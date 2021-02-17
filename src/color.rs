use std::hash::Hash;

use serde::Deserialize;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ColorError {
    #[error("only hex strings of length 3 or 6 are supported")]
    InvalidLength,
    #[error("values of the triplet must be valid hex")]
    InvalidHex(#[from] std::num::ParseIntError),
}

#[derive(Debug)]
pub struct ScaledColor {
    pub r: f64,
    pub g: f64,
    pub b: f64,
    pub a: f64,
}

#[derive(Debug)]
pub enum PerceivedLuminance {
    Light,
    Dark,
}

#[derive(Debug, Hash, Deserialize, Default)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl Color {
    pub fn from_hex(hex: &str) -> Result<Self, ColorError> {
        if hex.len() != 3 && hex.len() != 6 {
            return Err(ColorError::InvalidLength);
        }
        let mut fixed_length_hex = hex.to_lowercase();
        if hex.len() == 3 {
            fixed_length_hex = hex.chars().fold(String::new(), |prev, current| {
                format!("{}{}{}", prev, current, current)
            })
        }
        let red = u8::from_str_radix(&fixed_length_hex[0..2], 16)?;
        let green = u8::from_str_radix(&fixed_length_hex[2..4], 16)?;
        let blue = u8::from_str_radix(&fixed_length_hex[4..6], 16)?;

        Ok(Self {
            r: red,
            g: green,
            b: blue,
            a: 1u8,
        })
    }

    pub fn perceived_luminance(&self) -> PerceivedLuminance {
        let ScaledColor { r, g, b, .. } = self.to_scaled();
        let r = srgb_to_linear(r);
        let g = srgb_to_linear(g);
        let b = srgb_to_linear(b);
        let luminance = 0.2126 * r + 0.7152 * g + 0.0722 * b;
        let perceived_lum = luminance_to_perceived_luminance(luminance);
        if perceived_lum >= 80.0 {
            PerceivedLuminance::Light
        } else {
            PerceivedLuminance::Dark
        }
    }

    pub fn to_scaled(&self) -> ScaledColor {
        ScaledColor {
            r: self.r as f64 / 255.0,
            g: self.g as f64 / 255.0,
            b: self.b as f64 / 255.0,
            a: self.a as f64,
        }
    }
}

fn srgb_to_linear(channel: f64) -> f64 {
    if channel <= 0.04045 {
        channel / 12.92
    } else {
        f64::powf((channel + 0.055) / 1.055, 2.4)
    }
}

fn luminance_to_perceived_luminance(luminance: f64) -> f64 {
    if luminance <= (216.0 / 24389.0) {
        luminance * (24389.0 / 27.0)
    } else {
        f64::powf(luminance, 1.0 / 3.0) * 116.0 - 16.0
    }
}
