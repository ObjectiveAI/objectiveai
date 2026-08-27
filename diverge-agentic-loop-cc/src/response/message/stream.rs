//! The API's raw streaming events.

use serde::{Deserialize, Serialize};

use super::{ContentBlock, DeltaUsage, Message, StopReason, TextCitation};

/// `BetaRawMessageStreamEvent`: one event of a streamed API message,
/// discriminated by `type`. Six kinds, and the pairing is the API's
/// own: a message starts, blocks start and grow and stop inside it,
/// the message's tail deltas arrive, and it stops.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum StreamEvent {
    /// The message beginning: the whole [`Message`] envelope with
    /// empty content and a null stop reason.
    MessageStart {
        /// The message so far.
        message: Message,
    },
    /// The message's closing fields, once known.
    MessageDelta {
        /// What became known.
        delta: MessageDelta,
        /// Cumulative output tokens so far.
        usage: DeltaUsage,
    },
    /// The message is over.
    MessageStop,
    /// A content block beginning, at its index.
    ContentBlockStart {
        /// Where in the message's content it sits.
        index: u64,
        /// The block, possibly empty, to be grown by deltas.
        content_block: ContentBlock,
    },
    /// A content block growing.
    ContentBlockDelta {
        /// Which block is growing.
        index: u64,
        /// The growth itself.
        delta: ContentBlockDelta,
    },
    /// A content block finished.
    ContentBlockStop {
        /// Which block finished.
        index: u64,
    },
}

/// What a `message_delta` learned: the stop fields, in the same
/// vocabulary the whole message uses.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MessageDelta {
    /// Why generation stopped.
    pub stop_reason: Option<StopReason>,
    /// Which custom stop sequence fired, if one did.
    pub stop_sequence: Option<String>,
}

/// `content_block_delta`'s growth, discriminated by `type`. Five
/// kinds in the pinned SDK — one per thing a block can accumulate.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ContentBlockDelta {
    /// More text for a text block.
    TextDelta {
        /// The fragment.
        text: String,
    },
    /// More of a tool call's arguments, as a fragment of JSON text —
    /// the same not-yet-a-document shape the provider protocol's own
    /// tool call chunks carry.
    InputJsonDelta {
        /// The fragment.
        partial_json: String,
    },
    /// A citation landing on a text block.
    CitationsDelta {
        /// The citation, whole.
        citation: TextCitation,
    },
    /// More reasoning for a thinking block.
    ThinkingDelta {
        /// The fragment.
        thinking: String,
    },
    /// The thinking block's signature, at the end.
    SignatureDelta {
        /// The signature, whole.
        signature: String,
    },
}
