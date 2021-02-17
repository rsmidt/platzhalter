use crate::color::Color;
use once_cell::sync::OnceCell;
use serde::de::Error;
use serde::{Deserialize, Deserializer};

static HEX_RE: OnceCell<regex::Regex> = OnceCell::new();

pub fn color<'de, D>(deserializer: D) -> Result<Option<Color>, D::Error>
where
    D: Deserializer<'de>,
{
    let s: Option<String> = Option::deserialize(deserializer)?;
    if s.is_none() {
        return Ok(None);
    }
    let s = s.unwrap();
    let regex = HEX_RE
        .get_or_init(|| regex::Regex::new(r"^(([0-9a-fA-F]{2}){3}|([0-9a-fA-F]){3})$").unwrap());
    match regex.find(&s) {
        None => Ok(None),
        Some(m) => Ok(Some(Color::from_hex(m.as_str()).map_err(D::Error::custom)?)),
    }
}
