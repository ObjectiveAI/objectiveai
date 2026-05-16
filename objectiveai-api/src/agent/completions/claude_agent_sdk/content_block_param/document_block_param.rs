use objectiveai_sdk::agent::completions::message::File;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DocumentBlockParamType {
    Document,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Base64PDFSourceMediaType {
    #[serde(rename = "application/pdf")]
    ApplicationPdf,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Base64PDFSourceType {
    Base64,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Base64PDFSource {
    pub data: String,
    pub media_type: Base64PDFSourceMediaType,
    pub r#type: Base64PDFSourceType,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PlainTextSourceMediaType {
    #[serde(rename = "text/plain")]
    TextPlain,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PlainTextSourceType {
    Text,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct PlainTextSource {
    pub data: String,
    pub media_type: PlainTextSourceMediaType,
    pub r#type: PlainTextSourceType,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum URLPDFSourceType {
    Url,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct URLPDFSource {
    pub r#type: URLPDFSourceType,
    pub url: String,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ContentBlockSourceType {
    Content,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum ContentBlockSourceContent {
    Text(super::TextBlockParam),
    Image(super::ImageBlockParam),
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum ContentBlockSourceData {
    String(String),
    Blocks(Vec<ContentBlockSourceContent>),
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct ContentBlockSource {
    pub content: ContentBlockSourceData,
    pub r#type: ContentBlockSourceType,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum DocumentSource {
    Base64PDF(Base64PDFSource),
    PlainText(PlainTextSource),
    ContentBlock(ContentBlockSource),
    URLPDF(URLPDFSource),
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct DocumentBlockParam {
    pub source: DocumentSource,
    pub r#type: DocumentBlockParamType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache_control: Option<super::CacheControlEphemeral>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub citations: Option<super::CitationsConfigParam>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
}

impl TryFrom<&File> for DocumentBlockParam {
    type Error = String;

    fn try_from(file: &File) -> Result<Self, Self::Error> {
        if let Some(file_url) = &file.file_url {
            return Ok(DocumentBlockParam {
                r#type: DocumentBlockParamType::Document,
                source: DocumentSource::URLPDF(URLPDFSource {
                    r#type: URLPDFSourceType::Url,
                    url: file_url.clone(),
                }),
                cache_control: None,
                citations: None,
                context: None,
                title: file.filename.clone(),
            });
        }

        if let Some(file_data) = &file.file_data {
            let is_pdf = file
                .filename
                .as_ref()
                .map(|n| n.to_lowercase().ends_with(".pdf"))
                .unwrap_or(false);

            let source = if is_pdf {
                DocumentSource::Base64PDF(Base64PDFSource {
                    r#type: Base64PDFSourceType::Base64,
                    data: file_data.clone(),
                    media_type: Base64PDFSourceMediaType::ApplicationPdf,
                })
            } else {
                DocumentSource::PlainText(PlainTextSource {
                    r#type: PlainTextSourceType::Text,
                    data: file_data.clone(),
                    media_type: PlainTextSourceMediaType::TextPlain,
                })
            };

            return Ok(DocumentBlockParam {
                r#type: DocumentBlockParamType::Document,
                source,
                cache_control: None,
                citations: None,
                context: None,
                title: file.filename.clone(),
            });
        }

        let desc = file
            .filename
            .as_deref()
            .or(file.file_id.as_deref())
            .unwrap_or("unknown");
        Err(format!("unsupported file: no data or URL provided ({desc})"))
    }
}
