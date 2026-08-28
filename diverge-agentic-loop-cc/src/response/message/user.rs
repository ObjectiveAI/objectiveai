//! The request-side message the `user` records replay.

use serde::Deserialize;

use super::content_block::{
    BashCodeExecutionToolResultType, CodeExecutionToolResultType,
    ContainerUploadType, RedactedThinkingType, ServerToolUseType,
    TextCitation, TextEditorCodeExecutionToolResultType, TextType,
    ThinkingType, ToolSearchToolResultType, ToolUseType,
    WebFetchToolResultType, WebSearchToolResultType,
};
use super::caller::Caller;
use super::tool_results::{
    BashCodeExecutionToolResultContent, CodeExecutionToolResultContent,
    TextEditorCodeExecutionToolResultContent, ToolReferenceType,
    ToolSearchToolResultContent, WebFetchResultType,
    WebFetchToolResultError, WebSearchToolResultContent,
};

/// What a `user` record wraps: the API's own message params,
/// `{role, content}` and nothing else.
///
/// The content vocabulary is the REQUEST one, wider than the
/// response's four blocks: a user turn can carry images, documents,
/// and — the case the loop lives on — tool results.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct UserMessage {
    /// Always `user`.
    pub role: UserRole,
    /// The content: bare text, or blocks.
    pub content: UserContent,
}

/// [`UserMessage`]'s role literal.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Deserialize,
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
#[derive(Debug, Clone, PartialEq, Deserialize)]
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
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(untagged)]
pub enum ContentBlockParam {
    /// Text.
    Text {
        /// Always `text`.
        r#type: TextType,
        /// The text itself.
        text: String,
        /// Cache-control marker, if any — optional AND nullable in
        /// the SDK, both spellings landing as `None`.
        cache_control: Option<CacheControl>,
        /// Citations attached to the text, if any. The param
        /// locations carry the same fields as the response ones, so
        /// one type serves both. Optional AND nullable in the SDK,
        /// like `cache_control`.
        citations: Option<Vec<TextCitation>>,
    },
    /// An image, by bytes or by URL.
    Image {
        /// Always `image`.
        r#type: ImageType,
        /// Where the image comes from.
        source: ImageSource,
        /// Cache-control marker, if any — optional AND nullable in
        /// the SDK, both spellings landing as `None`.
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
        /// Cache-control marker, if any — optional AND nullable in
        /// the SDK, both spellings landing as `None`.
        cache_control: Option<CacheControl>,
    },
    /// A tool's answer — what the loop's tool results ride in.
    ToolResult {
        /// Always `tool_result`.
        r#type: ToolResultType,
        /// The call being answered.
        tool_use_id: String,
        /// What the tool said: absent, bare text, or blocks — the
        /// five kinds a tool result may quote.
        content: Option<ToolResultParamContent>,
        /// Whether the tool considers itself to have failed.
        is_error: Option<bool>,
        /// Cache-control marker, if any — optional AND nullable in
        /// the SDK, both spellings landing as `None`.
        cache_control: Option<CacheControl>,
    },
    /// A server-side tool call quoted back.
    ServerToolUse {
        /// Always `server_tool_use`.
        r#type: ServerToolUseType,
        /// The call's id.
        id: String,
        /// Which server tool — an API-owned vocabulary, open.
        name: String,
        /// The arguments, whatever the tool's schema says.
        input: serde_json::Value,
        /// Cache-control marker, if any.
        cache_control: Option<CacheControl>,
        /// Who made the call, when a server-side tool did.
        caller: Option<Caller>,
    },
    /// A web search's answer quoted back — the same content union
    /// the response side carries.
    WebSearchToolResult {
        /// Always `web_search_tool_result`.
        r#type: WebSearchToolResultType,
        /// The results or the error.
        content: WebSearchToolResultContent,
        /// The call being answered.
        tool_use_id: String,
        /// Cache-control marker, if any.
        cache_control: Option<CacheControl>,
        /// Who made the call, when a server-side tool did.
        caller: Option<Caller>,
    },
    /// A web fetch's answer quoted back — its document is the
    /// REQUEST-side one, which is where this union parts from the
    /// response side's.
    WebFetchToolResult {
        /// Always `web_fetch_tool_result`.
        r#type: WebFetchToolResultType,
        /// The document or the error.
        content: WebFetchToolResultParamContent,
        /// The call being answered.
        tool_use_id: String,
        /// Cache-control marker, if any.
        cache_control: Option<CacheControl>,
        /// Who made the call, when a server-side tool did.
        caller: Option<Caller>,
    },
    /// A code execution's answer quoted back.
    CodeExecutionToolResult {
        /// Always `code_execution_tool_result`.
        r#type: CodeExecutionToolResultType,
        /// The result, its encrypted twin, or the error.
        content: CodeExecutionToolResultContent,
        /// The call being answered.
        tool_use_id: String,
        /// Cache-control marker, if any.
        cache_control: Option<CacheControl>,
    },
    /// A bash execution's answer quoted back.
    BashCodeExecutionToolResult {
        /// Always `bash_code_execution_tool_result`.
        r#type: BashCodeExecutionToolResultType,
        /// The result or the error.
        content: BashCodeExecutionToolResultContent,
        /// The call being answered.
        tool_use_id: String,
        /// Cache-control marker, if any.
        cache_control: Option<CacheControl>,
    },
    /// A text-editor execution's answer quoted back.
    TextEditorCodeExecutionToolResult {
        /// Always `text_editor_code_execution_tool_result`.
        r#type: TextEditorCodeExecutionToolResultType,
        /// One of three result shapes, or the error.
        content: TextEditorCodeExecutionToolResultContent,
        /// The call being answered.
        tool_use_id: String,
        /// Cache-control marker, if any.
        cache_control: Option<CacheControl>,
    },
    /// A tool search's answer quoted back.
    ToolSearchToolResult {
        /// Always `tool_search_tool_result`.
        r#type: ToolSearchToolResultType,
        /// The references found, or the error.
        content: ToolSearchToolResultContent,
        /// The call being answered.
        tool_use_id: String,
        /// Cache-control marker, if any.
        cache_control: Option<CacheControl>,
    },
    /// A container upload quoted back.
    ContainerUpload {
        /// Always `container_upload`.
        r#type: ContainerUploadType,
        /// The file, by id.
        file_id: String,
        /// Cache-control marker, if any.
        cache_control: Option<CacheControl>,
    },
    /// A document, by bytes, text, content, or URL.
    Document(DocumentBlockParam),
    /// A search result quoted in, citable.
    SearchResult(SearchResultBlockParam),
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
    Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum ImageType {
    /// The only value.
    #[default]
    Image,
}

/// The `tool_result` literal.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum ToolResultType {
    /// The only value.
    #[default]
    ToolResult,
}

/// The `document` literal.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum DocumentType {
    /// The only value.
    #[default]
    Document,
}

/// Where an image's bytes come from. Untagged with literal markers.
#[derive(Debug, Clone, PartialEq, Deserialize)]
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
    Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum Base64Type {
    /// The only value.
    #[default]
    Base64,
}

/// The `url` literal.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum UrlType {
    /// The only value.
    #[default]
    Url,
}

/// The `content` literal.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum ContentType {
    /// The only value.
    #[default]
    Content,
}

/// The image formats the API accepts inline.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize,
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
#[derive(Debug, Clone, PartialEq, Deserialize)]
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
        content: DocumentSourceContent,
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
    Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Deserialize,
)]
pub enum PdfMediaType {
    /// The only value.
    #[default]
    #[serde(rename = "application/pdf")]
    ApplicationPdf,
}

/// The one media type a plain-text document can be.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Deserialize,
)]
pub enum PlainTextMediaType {
    /// The only value.
    #[default]
    #[serde(rename = "text/plain")]
    TextPlain,
}

/// What a `content`-sourced document holds: a bare string, or text
/// and image blocks.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(untagged)]
pub enum DocumentSourceContent {
    /// The whole content as one string.
    Text(String),
    /// The content as text and image blocks.
    Blocks(Vec<TextOrImageParam>),
}

/// What a tool result says: a bare string, or blocks of the five
/// kinds the current SDK admits there.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(untagged)]
pub enum ToolResultParamContent {
    /// The whole content as one string.
    Text(String),
    /// The content as blocks.
    Blocks(Vec<ToolResultContentBlock>),
}

/// One block inside a tool result's content — the current SDK's
/// five kinds.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(untagged)]
pub enum ToolResultContentBlock {
    /// Text.
    Text {
        /// Always `text`.
        r#type: TextType,
        /// The text itself.
        text: String,
        /// Cache-control marker, if any.
        cache_control: Option<CacheControl>,
        /// Citations attached to the text, if any.
        citations: Option<Vec<TextCitation>>,
    },
    /// An image.
    Image {
        /// Always `image`.
        r#type: ImageType,
        /// Where the image comes from.
        source: ImageSource,
        /// Cache-control marker, if any.
        cache_control: Option<CacheControl>,
    },
    /// A search result.
    SearchResult(SearchResultBlockParam),
    /// A document.
    Document(DocumentBlockParam),
    /// A reference to a tool.
    ToolReference(ToolReferenceBlockParam),
}

/// A request-side document, standalone — the [`ContentBlockParam`]
/// variant's body, named so a web fetch's quoted answer and a tool
/// result's content can carry one too.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct DocumentBlockParam {
    /// Always `document`.
    pub r#type: DocumentType,
    /// Where the document comes from.
    pub source: DocumentSource,
    /// Cache-control marker, if any — optional AND nullable in
    /// the SDK, both spellings landing as `None`.
    pub cache_control: Option<CacheControl>,
    /// Whether citations are enabled for it.
    pub citations: Option<CitationsConfig>,
    /// Context about the document, kept out of the cited text.
    /// Optional AND nullable in the SDK, like `cache_control`.
    pub context: Option<String>,
    /// The document's title. Optional AND nullable, likewise.
    pub title: Option<String>,
}

/// A search result quoted into the conversation, citable.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct SearchResultBlockParam {
    /// Always `search_result`.
    pub r#type: SearchResultType,
    /// The result's content — text blocks on every wire the SDK
    /// admits.
    pub content: Vec<TextOrImageParam>,
    /// Where the result came from.
    pub source: String,
    /// The result's title.
    pub title: String,
    /// Cache-control marker, if any.
    pub cache_control: Option<CacheControl>,
    /// Whether citations are enabled for it.
    pub citations: Option<CitationsConfig>,
}

/// The `search_result` literal.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum SearchResultType {
    /// The only value.
    #[default]
    SearchResult,
}

/// A reference to a tool, quoted back with cache control.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct ToolReferenceBlockParam {
    /// Always `tool_reference`.
    pub r#type: ToolReferenceType,
    /// The tool's name.
    pub tool_name: String,
    /// Cache-control marker, if any.
    pub cache_control: Option<CacheControl>,
}

/// A quoted web fetch's content: the document or the error — the
/// request side's twin of the response union, differing exactly in
/// which document rides inside.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(untagged)]
pub enum WebFetchToolResultParamContent {
    /// The fetch failed.
    Error(WebFetchToolResultError),
    /// The fetched page.
    Result(WebFetchBlockParam),
}

/// The fetched page, request-side: a request document and its URL.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct WebFetchBlockParam {
    /// Always `web_fetch_result`.
    pub r#type: WebFetchResultType,
    /// The page, as a request-side document.
    pub content: DocumentBlockParam,
    /// When it was fetched, when known.
    pub retrieved_at: Option<String>,
    /// The URL fetched.
    pub url: String,
}

/// The narrow block union inside a [`DocumentSourceContent`]: text
/// or an image, with the same fields those blocks have anywhere
/// else. Untagged with the shared literal markers.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(untagged)]
pub enum TextOrImageParam {
    /// Text.
    Text {
        /// Always `text`.
        r#type: TextType,
        /// The text itself.
        text: String,
        /// Cache-control marker, if any — optional AND nullable in
        /// the SDK, both spellings landing as `None`.
        cache_control: Option<CacheControl>,
        /// Citations attached to the text, if any. Optional AND
        /// nullable in the SDK, like `cache_control`.
        citations: Option<Vec<TextCitation>>,
    },
    /// An image.
    Image {
        /// Always `image`.
        r#type: ImageType,
        /// Where the image comes from.
        source: ImageSource,
        /// Cache-control marker, if any — optional AND nullable in
        /// the SDK, both spellings landing as `None`.
        cache_control: Option<CacheControl>,
    },
}

/// `CacheControlEphemeral`: the cache marker — the pinned SDK's bare
/// `{type}` and the current SDK's `{type, ttl}` in one shape. With
/// no [`ttl`](Self::ttl) it is exactly the old form, so both
/// vintages of the wire read correctly.
#[derive(
    Debug, Clone, Default, PartialEq, Eq, Hash, Deserialize,
)]
pub struct CacheControl {
    /// Always `ephemeral`.
    pub r#type: CacheControlType,
    /// How long to cache for, when said.
    pub ttl: Option<CacheControlTtl>,
}

/// The `ephemeral` literal.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum CacheControlType {
    /// The only value.
    #[default]
    Ephemeral,
}

/// A cache lifetime — `5m` and `1h` today, and an API-owned
/// vocabulary, so anything newer is preserved verbatim in
/// [`Other`](Self::Other). Serde reads it through [`String`], the
/// flat-open-enum treatment.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Deserialize)]
#[serde(from = "String")]
pub enum CacheControlTtl {
    /// Five minutes.
    FiveMinutes,
    /// One hour.
    OneHour,
    /// A lifetime newer than this crate, preserved verbatim.
    Other(String),
}

impl From<String> for CacheControlTtl {
    fn from(value: String) -> Self {
        match value.as_str() {
            "5m" => CacheControlTtl::FiveMinutes,
            "1h" => CacheControlTtl::OneHour,
            _ => CacheControlTtl::Other(value),
        }
    }
}


/// `CitationsConfigParam`: whether a document's citations are on.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Deserialize,
)]
pub struct CitationsConfig {
    /// Whether citations are enabled.
    pub enabled: Option<bool>,
}
