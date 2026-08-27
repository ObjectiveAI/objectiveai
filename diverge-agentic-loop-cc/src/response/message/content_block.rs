//! What the model produced, one block at a time.

use serde::{Deserialize, Serialize};

/// `BetaContentBlock`: one block of an assistant message's content.
///
/// Four kinds in `@anthropic-ai/sdk@0.39.0`, discriminated by `type`.
/// A newer API can produce blocks this version does not name, and a
/// record carrying one fails to parse — deliberately, per this
/// module's strictness: reading somebody else's bytes as one of these
/// four would be worse than saying so.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ContentBlock {
    /// Text, with whatever citations support it.
    Text {
        /// The text itself.
        text: String,
        /// Citations supporting the block; `null` when citations are
        /// not in play at all.
        citations: Option<Vec<TextCitation>>,
    },
    /// The model invoking a tool.
    ToolUse {
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
        /// The reasoning text.
        thinking: String,
        /// The integrity signature over it.
        signature: String,
    },
    /// Reasoning the API withheld, still replayable.
    RedactedThinking {
        /// The opaque encrypted payload.
        data: String,
    },
}

/// `BetaTextCitation`: where a piece of text came from,
/// discriminated by `type`.
///
/// Three location vocabularies for three source shapes: character
/// offsets into plain text, pages of a PDF, and block indices of
/// custom content. The `Param` unions on the request side carry the
/// same fields, so these serve both directions.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum TextCitation {
    /// A span of characters in a plain-text document.
    CharLocation {
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
