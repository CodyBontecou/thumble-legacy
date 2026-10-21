//! Typed rich-background payloads. File IO remains with the native CLI; local
//! MCP supplies the same bounded bytes, never an arbitrary host path.
use super::{validate_element_fill, ConfigurationOperationError, ElementFill};
use base64::{engine::general_purpose::STANDARD, Engine};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum BackgroundFill {
    Paint(ElementFill),
    Image(BackgroundImageFill),
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum BackgroundImageKind {
    Image,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum BackgroundImageMode {
    Fill,
    Fit,
    Tile,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BackgroundImageFill {
    pub kind: BackgroundImageKind,
    pub data_base64: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub file_name: Option<String>,
    pub content_mode: BackgroundImageMode,
}
impl BackgroundFill {
    pub(crate) fn validate(&self) -> Result<(), ConfigurationOperationError> {
        match self {
            Self::Paint(fill) => validate_element_fill(fill),
            Self::Image(image) => {
                const MAXIMUM: usize = 2_500_000;
                if image.data_base64.len() > MAXIMUM.div_ceil(3) * 4 {
                    return Err(ConfigurationOperationError::InvalidStyle);
                }
                let data = STANDARD
                    .decode(&image.data_base64)
                    .map_err(|_| ConfigurationOperationError::InvalidStyle)?;
                if data.is_empty()
                    || data.len() > MAXIMUM
                    || STANDARD.encode(&data) != image.data_base64
                    || image.file_name.as_ref().is_some_and(|name| {
                        name.is_empty()
                            || name.len() > 512
                            || name.contains(['/', '\\'])
                            || name.chars().any(char::is_control)
                    })
                {
                    return Err(ConfigurationOperationError::InvalidStyle);
                }
                Ok(())
            }
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn images_require_bounded_canonical_bytes_and_local_display_names() {
        let mut image = BackgroundImageFill {
            kind: BackgroundImageKind::Image,
            data_base64: STANDARD.encode([1, 2, 3]),
            file_name: Some("image.png".into()),
            content_mode: BackgroundImageMode::Fit,
        };
        assert!(BackgroundFill::Image(image.clone()).validate().is_ok());
        for name in ["../image.png", "a\\b", "", "bad\nname"] {
            image.file_name = Some(name.into());
            assert!(BackgroundFill::Image(image.clone()).validate().is_err());
        }
        image.file_name = None;
        for encoded in ["".into(), "AQI".into(), STANDARD.encode(vec![0; 2_500_001])] {
            image.data_base64 = encoded;
            assert!(BackgroundFill::Image(image.clone()).validate().is_err());
        }
        assert!(serde_json::from_str::<BackgroundFill>(
            r#"{"kind":"image","dataBase64":"AQID","contentMode":"fit","path":"/tmp/image.png"}"#
        )
        .is_err());
    }
}
