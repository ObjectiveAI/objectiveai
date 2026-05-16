use objectiveai_sdk::agent::completions::message::ImageUrl;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ImageBlockParamType {
    Image,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum Base64ImageSourceMediaType {
    #[serde(rename = "image/jpeg")]
    ImageJpeg,
    #[serde(rename = "image/png")]
    ImagePng,
    #[serde(rename = "image/gif")]
    ImageGif,
    #[serde(rename = "image/webp")]
    ImageWebp,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Base64ImageSourceType {
    Base64,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Base64ImageSource {
    pub data: String,
    pub media_type: Base64ImageSourceMediaType,
    pub r#type: Base64ImageSourceType,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum URLImageSourceType {
    Url,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct URLImageSource {
    pub r#type: URLImageSourceType,
    pub url: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum ImageSource {
    Base64(Base64ImageSource),
    URL(URLImageSource),
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct ImageBlockParam {
    pub source: ImageSource,
    pub r#type: ImageBlockParamType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache_control: Option<super::CacheControlEphemeral>,
}

impl TryFrom<&ImageUrl> for ImageBlockParam {
    type Error = String;

    fn try_from(image_url: &ImageUrl) -> Result<Self, Self::Error> {
        let url = &image_url.url;
        if url.starts_with("data:") {
            let comma_index = url.find(',').ok_or("invalid data URI: no comma")?;
            let meta = &url[5..comma_index];
            let raw_media_type = meta.split(';').next().unwrap_or("");
            let media_type = match raw_media_type {
                "image/jpeg" | "image/jpg" => Base64ImageSourceMediaType::ImageJpeg,
                "image/png" => Base64ImageSourceMediaType::ImagePng,
                "image/gif" => Base64ImageSourceMediaType::ImageGif,
                "image/webp" => Base64ImageSourceMediaType::ImageWebp,
                _ => return Err(format!("unsupported image media type: {raw_media_type}")),
            };
            let data = url[comma_index + 1..].to_owned();
            Ok(ImageBlockParam {
                r#type: ImageBlockParamType::Image,
                source: ImageSource::Base64(Base64ImageSource {
                    r#type: Base64ImageSourceType::Base64,
                    data,
                    media_type,
                }),
                cache_control: None,
            })
        } else {
            Ok(ImageBlockParam {
                r#type: ImageBlockParamType::Image,
                source: ImageSource::URL(URLImageSource {
                    r#type: URLImageSourceType::Url,
                    url: url.clone(),
                }),
                cache_control: None,
            })
        }
    }
}
