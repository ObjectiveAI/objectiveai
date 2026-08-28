//! What the model produced, one block at a time.

use serde::{Deserialize, Serialize};

/// `BetaContentBlock`: one block of an assistant message's content.
///
/// Four kinds in `@anthropic-ai/sdk@0.39.0`. Untagged, the way this
/// crate models unions: each member carries its own `type` literal
/// as a single-variant marker, so the object is self-describing
/// wherever it travels, and only the right variant can accept a
/// given literal. A newer API can produce blocks this version does
/// not name, and a record carrying one fails to parse —
/// deliberately: reading somebody else's bytes as one of these four
/// would be worse than saying so.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ContentBlock {
    /// Text, with whatever citations support it.
    Text {
        /// Always `text`.
        r#type: TextType,
        /// The text itself.
        text: String,
        /// Citations supporting the block; `null` when citations are
        /// not in play at all.
        citations: Option<Vec<TextCitation>>,
    },
    /// The model invoking a tool.
    ToolUse {
        /// Always `tool_use`.
        r#type: ToolUseType,
        /// The call's id, quoted by the matching tool result.
        id: String,
        /// The tool being called.
        name: String,
        /// The arguments, whatever the tool's schema says they are —
        /// `unknown` in the SDK, and kept that way.
        input: serde_json::Value,
    },
    /// The model's reasoning, with the signature that lets it be
    /// replayed.
    Thinking {
        /// Always `thinking`.
        r#type: ThinkingType,
        /// The reasoning text.
        thinking: String,
        /// The integrity signature over it.
        signature: String,
    },
    /// Reasoning the API withheld, still replayable.
    RedactedThinking {
        /// Always `redacted_thinking`.
        r#type: RedactedThinkingType,
        /// The opaque encrypted payload.
        data: String,
    },
}

/// The `text` literal.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum TextType {
    /// The only value.
    #[default]
    Text,
}

/// The `tool_use` literal.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum ToolUseType {
    /// The only value.
    #[default]
    ToolUse,
}

/// The `thinking` literal.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum ThinkingType {
    /// The only value.
    #[default]
    Thinking,
}

/// The `redacted_thinking` literal.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum RedactedThinkingType {
    /// The only value.
    #[default]
    RedactedThinking,
}

/// `BetaTextCitation`: where a piece of text came from.
///
/// Three location vocabularies for three source shapes: character
/// offsets into plain text, pages of a PDF, and block indices of
/// custom content. The `Param` unions on the request side carry the
/// same fields, so these serve both directions. Untagged with
/// literal markers, like every union here.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum TextCitation {
    /// A span of characters in a plain-text document.
    CharLocation {
        /// Always `char_location`.
        r#type: CharLocationType,
        /// The text being cited.
        cited_text: String,
        /// Which document, by request order.
        document_index: u64,
        /// The document's title, if it had one.
        document_title: Option<String>,
        /// First cited character.
        start_char_index: u64,
        /// One past the last cited character.
        end_char_index: u64,
    },
    /// A span of pages in a PDF.
    PageLocation {
        /// Always `page_location`.
        r#type: PageLocationType,
        /// The text being cited.
        cited_text: String,
        /// Which document, by request order.
        document_index: u64,
        /// The document's title, if it had one.
        document_title: Option<String>,
        /// First cited page.
        start_page_number: u64,
        /// One past the last cited page.
        end_page_number: u64,
    },
    /// A span of blocks in custom content.
    ContentBlockLocation {
        /// Always `content_block_location`.
        r#type: ContentBlockLocationType,
        /// The text being cited.
        cited_text: String,
        /// Which document, by request order.
        document_index: u64,
        /// The document's title, if it had one.
        document_title: Option<String>,
        /// First cited block.
        start_block_index: u64,
        /// One past the last cited block.
        end_block_index: u64,
    },
}

/// The `char_location` literal.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum CharLocationType {
    /// The only value.
    #[default]
    CharLocation,
}

/// The `page_location` literal.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum PageLocationType {
    /// The only value.
    #[default]
    PageLocation,
}

/// The `content_block_location` literal.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum ContentBlockLocationType {
    /// The only value.
    #[default]
    ContentBlockLocation,
}
