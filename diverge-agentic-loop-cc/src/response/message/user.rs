//! The request-side message the `user` records replay.

use serde::{Deserialize, Serialize};

use super::content_block::{
    RedactedThinkingType, TextCitation, TextType, ThinkingType, ToolUseType,
};

/// What a `user` record wraps: the API's own message params,
/// `{role, content}` and nothing else.
///
/// The content vocabulary is the REQUEST one, wider than the
/// response's four blocks: a user turn can carry images, documents,
/// and — the case the loop lives on — tool results.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UserMessage {
    /// Always `user`.
    pub role: UserRole,
    /// The content: bare text, or blocks.
    pub content: UserContent,
}

/// [`UserMessage`]'s role literal.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum UserRole {
    /// The only value.
    #[default]
    User,
}

/// A user message's content: the API accepts a bare string or an
/// array of blocks, and Claude Code's records carry both — its own
/// normalization wraps strings into text blocks for SPLITTING, but
/// what lands on the wire is one block per record either way, and
/// synthetic records can still say a string.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum UserContent {
    /// The whole content as one string.
    Text(String),
    /// The content as blocks — on stdout, one per record.
    Blocks(Vec<ContentBlockParam>),
}

/// `ContentBlockParam`: one block of a request-side message. Seven
/// kinds in `@anthropic-ai/sdk@0.39.0`. Untagged with literal
/// markers, like every union here — the markers are shared with the
/// response blocks where the literals coincide.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ContentBlockParam {
    /// Text.
    Text {
        /// Always `text`.
        r#type: TextType,
        /// The text itself.
        text: String,
        /// Cache-control marker, if any.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        cache_control: Option<CacheControl>,
        /// Citations attached to the text, if any. The param
        /// locations carry the same fields as the response ones, so
        /// one type serves both.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        citations: Option<Vec<TextCitation>>,
    },
    /// An image, by bytes or by URL.
    Image {
        /// Always `image`.
        r#type: ImageType,
        /// Where the image comes from.
        source: ImageSource,
        /// Cache-control marker, if any.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        cache_control: Option<CacheControl>,
    },
    /// A tool call quoted back — a user turn can restate one.
    ToolUse {
        /// Always `tool_use`.
        r#type: ToolUseType,
        /// The call's id.
        id: String,
        /// The tool called.
        name: String,
        /// The arguments, whatever the tool says they are.
        input: serde_json::Value,
        /// Cache-control marker, if any.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        cache_control: Option<CacheControl>,
    },
    /// A tool's answer — what the loop's tool results ride in.
    ToolResult {
        /// Always `tool_result`.
        r#type: ToolResultType,
        /// The call being answered.
        tool_use_id: String,
        /// What the tool said: absent, bare text, or text-and-image
        /// blocks.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        content: Option<ToolResultContent>,
        /// Whether the tool considers itself to have failed.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        is_error: Option<bool>,
        /// Cache-control marker, if any.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        cache_control: Option<CacheControl>,
    },
    /// A document, by bytes, text, content, or URL.
    Document {
        /// Always `document`.
        r#type: DocumentType,
        /// Where the document comes from.
        source: DocumentSource,
        /// Cache-control marker, if any.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        cache_control: Option<CacheControl>,
        /// Whether citations are enabled for it.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        citations: Option<CitationsConfig>,
        /// Context about the document, kept out of the cited text.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        context: Option<String>,
        /// The document's title.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        title: Option<String>,
    },
    /// Reasoning quoted back for replay.
    Thinking {
        /// Always `thinking`.
        r#type: ThinkingType,
        /// The reasoning text.
        thinking: String,
        /// The integrity signature over it.
        signature: String,
    },
    /// Withheld reasoning quoted back for replay.
    RedactedThinking {
        /// Always `redacted_thinking`.
        r#type: RedactedThinkingType,
        /// The opaque encrypted payload.
        data: String,
    },
}

/// The `image` literal.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum ImageType {
    /// The only value.
    #[default]
    Image,
}

/// The `tool_result` literal.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum ToolResultType {
    /// The only value.
    #[default]
    ToolResult,
}

/// The `document` literal.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum DocumentType {
    /// The only value.
    #[default]
    Document,
}

/// Where an image's bytes come from. Untagged with literal markers.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ImageSource {
    /// Inline, base64.
    Base64 {
        /// Always `base64`.
        r#type: Base64Type,
        /// The bytes, base64.
        data: String,
        /// Which image format the bytes are.
        media_type: ImageMediaType,
    },
    /// By URL, fetched by the API.
    Url {
        /// Always `url`.
        r#type: UrlType,
        /// The image's URL.
        url: String,
    },
}

/// The `base64` literal.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum Base64Type {
    /// The only value.
    #[default]
    Base64,
}

/// The `url` literal.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum UrlType {
    /// The only value.
    #[default]
    Url,
}

/// The `content` literal.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum ContentType {
    /// The only value.
    #[default]
    Content,
}

/// The image formats the API accepts inline.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize,
)]
pub enum ImageMediaType {
    /// JPEG.
    #[serde(rename = "image/jpeg")]
    Jpeg,
    /// PNG.
    #[serde(rename = "image/png")]
    Png,
    /// GIF.
    #[serde(rename = "image/gif")]
    Gif,
    /// WebP.
    #[serde(rename = "image/webp")]
    Webp,
}

/// Where a document comes from — four sources with four literals:
/// `base64` is a PDF's bytes, `text` is plain text, `content` is
/// custom blocks, `url` is fetched. Untagged with literal markers;
/// `text` shares the text blocks' marker because it is the same
/// literal.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum DocumentSource {
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
    /// Custom content: a string or text-and-image blocks.
    Content {
        /// Always `content`.
        r#type: ContentType,
        /// The content itself.
        content: ToolResultContent,
    },
    /// By URL, fetched by the API.
    Url {
        /// Always `url`.
        r#type: UrlType,
        /// The document's URL.
        url: String,
    },
}

/// The one media type a base64 document can be.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize,
)]
pub enum PdfMediaType {
    /// The only value.
    #[default]
    #[serde(rename = "application/pdf")]
    ApplicationPdf,
}

/// The one media type a plain-text document can be.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize,
)]
pub enum PlainTextMediaType {
    /// The only value.
    #[default]
    #[serde(rename = "text/plain")]
    TextPlain,
}

/// Text-or-blocks content: what a tool result says, and what a
/// `content`-sourced document holds — the SDK gives both the same
/// `string | Array<text | image>` shape.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ToolResultContent {
    /// The whole content as one string.
    Text(String),
    /// The content as text and image blocks.
    Blocks(Vec<TextOrImageParam>),
}

/// The narrow block union inside a [`ToolResultContent`]: text or an
/// image, with the same fields those blocks have anywhere else.
/// Untagged with the shared literal markers.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum TextOrImageParam {
    /// Text.
    Text {
        /// Always `text`.
        r#type: TextType,
        /// The text itself.
        text: String,
        /// Cache-control marker, if any.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        cache_control: Option<CacheControl>,
        /// Citations attached to the text, if any.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        citations: Option<Vec<TextCitation>>,
    },
    /// An image.
    Image {
        /// Always `image`.
        r#type: ImageType,
        /// Where the image comes from.
        source: ImageSource,
        /// Cache-control marker, if any.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        cache_control: Option<CacheControl>,
    },
}

/// `CacheControlEphemeral`: the cache marker, an object whose only
/// field is its own `type` literal — already self-describing, so it
/// needs no untagging.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize,
)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum CacheControl {
    /// The only kind.
    #[default]
    Ephemeral,
}

/// `CitationsConfigParam`: whether a document's citations are on.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize,
)]
pub struct CitationsConfig {
    /// Whether citations are enabled.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub enabled: Option<bool>,
}
