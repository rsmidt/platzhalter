FROM rust:1.63-bullseye as builder

RUN USER=root cargo new --bin platzhalter
WORKDIR ./platzhalter
COPY ./Cargo.toml ./Cargo.toml
RUN cargo build --release
RUN rm src/*.rs

ADD . ./

RUN rm ./target/release/deps/platzhalter*
RUN cargo build --release


FROM debian:bullseye-slim
ARG APP=/usr/src/app

RUN apt-get update \
    && apt-get install -y ca-certificates tzdata libcairo2 libglib2.0-0 libcairo-gobject2 \
    && rm -rf /var/lib/apt/lists/*

EXPOSE 8000

ENV TZ=Etc/UTC \
    APP_USER=appuser

RUN groupadd $APP_USER \
    && useradd -g $APP_USER $APP_USER \
    && mkdir -p ${APP}

RUN apt list --installed glib

COPY --from=builder /platzhalter/target/release/platzhalter ${APP}/platzhalter

RUN chown -R $APP_USER:$APP_USER ${APP}

USER $APP_USER
WORKDIR ${APP}

CMD ["./platzhalter"]
