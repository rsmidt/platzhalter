use std::hash::Hash;

use actix_cors::Cors;
use actix_web::{App, error, get, HttpResponse, HttpServer, web::{self, Data}};
use actix_web_opentelemetry::{RequestMetricsBuilder, RequestTracing};
use cairo::{Context, FontSlant, FontWeight, Format, ImageSurface};
use opentelemetry::{global, KeyValue, sdk::trace as sdktrace};
use opentelemetry::global::shutdown_tracer_provider;
use opentelemetry::sdk::export::metrics::aggregation::{cumulative_temporality_selector, delta_temporality_selector};
use opentelemetry::sdk::metrics::selectors::simple::inexpensive;
use opentelemetry::sdk::Resource;
use opentelemetry_otlp::WithExportConfig;
use serde::Deserialize;
use sled::open;
use tracing;
use tracing::info;
use tracing_subscriber::{EnvFilter, Registry};
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

use crate::color::{Color, PerceivedLuminance};

mod color;
mod color_serde;
mod service;

static DIMENSION_RE: once_cell::sync::OnceCell<regex::Regex> = once_cell::sync::OnceCell::new();

#[derive(Debug, Hash, Deserialize)]
struct ImageConfig {
    #[serde(default)]
    #[serde(deserialize_with = "color_serde::color")]
    bg: Option<Color>,
    #[serde(default)]
    #[serde(deserialize_with = "color_serde::color")]
    br: Option<Color>,
    br_s: Option<u8>,
}

#[derive(Debug, Hash)]
pub struct ImageMeta<'a> {
    config: ImageConfig,
    raw_dimensions: &'a str,
}

#[get("/{dimensions}")]
async fn index(
    params: web::Path<String>,
    web::Query(image_config): web::Query<ImageConfig>,
    db: web::Data<sled::Db>,
) -> error::Result<HttpResponse> {
    let dimensions = params.into_inner();

    let regex = DIMENSION_RE.get_or_init(|| {
        regex::Regex::new(r"(?P<length>[1-9][0-9]+)x(?P<height>[1-9][0-9]+)").unwrap()
    });
    let caps = regex
        .captures(&dimensions)
        .ok_or_else(|| error::ErrorBadRequest("Invalid dimensions"))?;
    let length: i32 = caps["length"].parse().unwrap();
    let height: i32 = caps["height"].parse().unwrap();

    if length > 3000 || height > 3000 {
        return Err(error::ErrorBadRequest("max dimension is 3000x3000"));
    }

    let meta = ImageMeta {
        config: image_config,
        raw_dimensions: &dimensions,
    };

    if let Some(bytes) =
        service::get_from_db(db.get_ref(), &meta).map_err(error::ErrorInternalServerError)?
    {
        return Ok(HttpResponse::Ok().content_type("image/png").body(bytes));
    }

    let surface = ImageSurface::create(Format::ARgb32, length, height)
        .map_err(error::ErrorBadRequest)?;

    let context = Context::new(&surface).unwrap();
    let default_color = Color::from_hex("FFD8C2").unwrap();
    let bg_color = &meta.config.bg.as_ref().unwrap_or(&default_color);
    let bg_color_scaled = bg_color.to_scaled();
    context.set_source_rgb(bg_color_scaled.r, bg_color_scaled.g, bg_color_scaled.b);
    context.paint().unwrap();

    if let Some(border_size) = meta.config.br_s {
        let br_color = meta
            .config
            .br
            .as_ref()
            .unwrap_or(&Color::from_hex("000").unwrap())
            .to_scaled();
        context.set_source_rgb(br_color.r, br_color.g, br_color.b);
        context.rectangle(
            0f64,
            0f64,
            surface.width() as f64,
            surface.height() as f64,
        );
        context.set_line_width(border_size as f64);
        context.stroke().unwrap();
    }

    context.select_font_face("Sans", FontSlant::Normal, FontWeight::Bold);
    context.set_font_size(surface.width() as f64 / dimensions.len() as f64 * 1.2);

    let text_extents = context.text_extents(&dimensions).unwrap();
    let width = text_extents.width();
    let height = text_extents.height();
    let x_bearing = text_extents.x_bearing();
    let y_bearing = text_extents.y_bearing();
    let x = surface.width() as f64 / 2.0 - (width / 2.0 + x_bearing);
    let y = surface.height() as f64 / 2.0 - (height / 2.0 + y_bearing);
    context.move_to(x, y);
    let text_color = match bg_color.perceived_luminance() {
        PerceivedLuminance::Light => Color::from_hex("111827").unwrap(),
        PerceivedLuminance::Dark => Color::from_hex("F9FAFB").unwrap(),
    }
        .to_scaled();
    context.set_source_rgb(text_color.r, text_color.g, text_color.b);
    context.show_text(&dimensions).unwrap();

    if surface.width() >= 200 {
        let border_size: f64 = meta.config.br_s.unwrap_or(0).into();
        let powered_by_text = "powered by rsmidt.dev";
        context.select_font_face("Sans", FontSlant::Normal, FontWeight::Normal);
        let proposed_font_size = surface.width() as f64 / powered_by_text.len() as f64;
        context.set_font_size(proposed_font_size.clamp(12.0, 40.0));
        let powered_by_extents = context.text_extents(powered_by_text).unwrap();
        let x = surface.width() as f64 - powered_by_extents.width() - 5.0 - border_size / 1.5;
        let y =
            surface.height() as f64 + powered_by_extents.y_bearing() / 2.0 - border_size / 1.5;
        context.move_to(x, y);
        context.set_source_rgba(text_color.r, text_color.g, text_color.b, 0.5);
        context.show_text(powered_by_text).unwrap();
    }

    let mut bytes: Vec<u8> = Vec::new();
    surface.write_to_png(&mut bytes).expect("sdf");

    service::insert(&db, &meta, bytes.clone()).map_err(error::ErrorInternalServerError)?;

    Ok(HttpResponse::Ok().content_type("image/png").body(bytes))
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let log_env = std::env::var("RUST_LOG").ok();
    if log_env.is_none() {
        std::env::set_var("RUST_LOG", "actix_web=warn,platzhalter=info");
    }

    let tracer = opentelemetry_otlp::new_pipeline()
        .tracing()
        .with_exporter(opentelemetry_otlp::new_exporter().tonic().with_env())
        .with_trace_config(
            sdktrace::config().with_resource(Resource::new(vec![KeyValue::new(
                opentelemetry_semantic_conventions::resource::SERVICE_NAME,
                "platzhalter",
            )])),
        )
        .install_batch(opentelemetry::runtime::Tokio).expect("failed to set up tracer pipeline");

    Registry::default()
        .with(EnvFilter::try_from_default_env().expect("RUST_LOG not set"))
        .with(tracing_subscriber::fmt::layer())
        .with(tracing_opentelemetry::layer().with_tracer(tracer))
        .init();

    let meter = global::meter("platzhalter");
    let request_metrics = RequestMetricsBuilder::new().build(meter);

    info!("Starting metrics push");
    let cx = opentelemetry::Context::new();
    let metrics_ctrl = opentelemetry_otlp::new_pipeline()
        .metrics(inexpensive(), cumulative_temporality_selector(), opentelemetry::runtime::Tokio)
        .with_exporter(
            opentelemetry_otlp::new_exporter()
                .tonic()
                .with_env()
        )
        .build()
        .expect("failed to setup up metrics pipeline");

    metrics_ctrl
        .start(&cx, opentelemetry::runtime::Tokio)
        .expect("failed to start otlp metrics push");

    let host = std::env::var("PLATZHALTER_HOST")
        .unwrap_or_else(|_| "127.0.0.1:8080".to_owned());

    let db = Data::new(sled::open("platzhalter_db")?);

    info!("Starting platzhalter running on {host}");

    HttpServer::new(move || {
        let cors = Cors::default()
            .allow_any_origin()
            .allowed_methods(vec!["GET"])
            .max_age(3600);

        App::new()
            .wrap(cors)
            .wrap(RequestTracing::new())
            .wrap(request_metrics.clone())
            .app_data(Data::clone(&db))
            .service(
                web::resource("/favicon.ico")
                    .route(web::get().to(|| async { HttpResponse::NotFound().finish() })),
            )
            .service(index)
    })
        .bind(&host)?
        .run()
        .await?;

    shutdown_tracer_provider();

    info!("Stopping metrics controller");
    metrics_ctrl.stop(&cx)?;

    Ok(())
}
