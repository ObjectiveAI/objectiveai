//! The response-side document: what a web fetch hands back.
//!
//! Not a content block of its own — it rides inside a
//! [`web_fetch_tool_result`](super::ContentBlock::WebFetchToolResult).
//! Narrower than the request-side document: two sources, no URL, no
//! custom content.

use serde::Deserialize;

use super::{
    Base64Type, DocumentType, PdfMediaType, PlainTextMediaType, TextType,
};

/// A fetched document: source, title, and whether citations are on.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct DocumentBlock {
    /// Always `document`.
    pub r#type: DocumentType,
    /// Where the document's bytes are.
    pub source: ResponseDocumentSource,
    /// Whether citations are enabled for it; `null` when unsaid.
    pub citations: Option<CitationConfig>,
    /// The document's title, if it has one.
    pub title: Option<String>,
}

/// A fetched document's source: a PDF's bytes or plain text — the
/// response side's two, against the request side's four.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(untagged)]
pub enum ResponseDocumentSource {
    /// A PDF's bytes, base64.
    Base64 {
        /// Always `base64`.
        r#type: Base64Type,
        /// The bytes, base64.
        data: String,
        /// Always `application/pdf`.
        media_type: PdfMediaType,
    },
    /// Plain text.
    Text {
        /// Always `text`.
        r#type: TextType,
        /// The text itself.
        data: String,
        /// Always `text/plain`.
        media_type: PlainTextMediaType,
    },
}

/// Whether a fetched document's citations are on — the response
/// side's config, whose `enabled` the API always says.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize,
)]
pub struct CitationConfig {
    /// Whether citations are enabled.
    pub enabled: bool,
}
