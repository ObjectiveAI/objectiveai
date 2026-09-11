//! The API's raw streaming events.

use serde::Deserialize;

use super::{
    Container, ContentBlock, ContextManagementResponse, DeltaUsage, Message,
    StopReason, TextCitation,
};

/// `BetaRawMessageStreamEvent`: one event of a streamed API message.
/// Six kinds, and the pairing is the API's own: a message starts,
/// blocks start and grow and stop inside it, the message's tail
/// deltas arrive, and it stops. Untagged with literal markers, like
/// every union here.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(untagged)]
pub enum StreamEvent {
    /// The message beginning: the whole [`Message`] envelope with
    /// empty content and a null stop reason.
    MessageStart {
        /// Always `message_start`.
        r#type: MessageStartType,
        /// The message so far.
        message: Message,
    },
    /// The message's closing fields, once known.
    MessageDelta {
        /// Always `message_delta`.
        r#type: MessageDeltaType,
        /// What became known.
        delta: MessageDelta,
        /// The cumulative usage so far.
        usage: DeltaUsage,
        /// Context-management state, when an edit landed mid-stream.
        context_management: Option<ContextManagementResponse>,
    },
    /// The message is over.
    MessageStop {
        /// Always `message_stop`.
        r#type: MessageStopType,
    },
    /// A content block beginning, at its index.
    ContentBlockStart {
        /// Always `content_block_start`.
        r#type: ContentBlockStartType,
        /// Where in the message's content it sits.
        index: u64,
        /// The block, possibly empty, to be grown by deltas.
        content_block: ContentBlock,
    },
    /// A content block growing.
    ContentBlockDelta {
        /// Always `content_block_delta`.
        r#type: ContentBlockDeltaType,
        /// Which block is growing.
        index: u64,
        /// The growth itself.
        delta: ContentBlockDelta,
    },
    /// A content block finished.
    ContentBlockStop {
        /// Always `content_block_stop`.
        r#type: ContentBlockStopType,
        /// Which block finished.
        index: u64,
    },
}

/// The `message_start` literal.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum MessageStartType {
    /// The only value.
    #[default]
    MessageStart,
}

/// The `message_delta` literal.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum MessageDeltaType {
    /// The only value.
    #[default]
    MessageDelta,
}

/// The `message_stop` literal.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum MessageStopType {
    /// The only value.
    #[default]
    MessageStop,
}

/// The `content_block_start` literal.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum ContentBlockStartType {
    /// The only value.
    #[default]
    ContentBlockStart,
}

/// The `content_block_delta` literal.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum ContentBlockDeltaType {
    /// The only value.
    #[default]
    ContentBlockDelta,
}

/// The `content_block_stop` literal.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum ContentBlockStopType {
    /// The only value.
    #[default]
    ContentBlockStop,
}

/// What a `message_delta` learned: the stop fields, in the same
/// vocabulary the whole message uses — and the container, whose
/// expiry can move mid-stream.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct MessageDelta {
    /// Why generation stopped.
    pub stop_reason: Option<StopReason>,
    /// Which custom stop sequence fired, if one did.
    pub stop_sequence: Option<String>,
    /// The code-execution container, when its state changed.
    pub container: Option<Container>,
}

/// `content_block_delta`'s growth. Five kinds in the pinned SDK —
/// one per thing a block can accumulate. Untagged with literal
/// markers, like every union here.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(untagged)]
pub enum ContentBlockDelta {
    /// More text for a text block.
    TextDelta {
        /// Always `text_delta`.
        r#type: TextDeltaType,
        /// The fragment.
        text: String,
    },
    /// More of a tool call's arguments, as a fragment of JSON text —
    /// the same not-yet-a-document shape the provider protocol's own
    /// tool call chunks carry.
    InputJsonDelta {
        /// Always `input_json_delta`.
        r#type: InputJsonDeltaType,
        /// The fragment.
        partial_json: String,
    },
    /// A citation landing on a text block.
    CitationsDelta {
        /// Always `citations_delta`.
        r#type: CitationsDeltaType,
        /// The citation, whole.
        citation: TextCitation,
    },
    /// More reasoning for a thinking block.
    ThinkingDelta {
        /// Always `thinking_delta`.
        r#type: ThinkingDeltaType,
        /// The fragment.
        thinking: String,
    },
    /// The thinking block's signature, at the end.
    SignatureDelta {
        /// Always `signature_delta`.
        r#type: SignatureDeltaType,
        /// The signature, whole.
        signature: String,
    },
    /// More of a compaction block's summary.
    CompactionDelta {
        /// Always `compaction_delta`.
        r#type: CompactionDeltaType,
        /// The fragment, when carried.
        content: Option<String>,
    },
}

/// The `compaction_delta` literal.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum CompactionDeltaType {
    /// The only value.
    #[default]
    CompactionDelta,
}

/// The `text_delta` literal.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum TextDeltaType {
    /// The only value.
    #[default]
    TextDelta,
}

/// The `input_json_delta` literal.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum InputJsonDeltaType {
    /// The only value.
    #[default]
    InputJsonDelta,
}

/// The `citations_delta` literal.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum CitationsDeltaType {
    /// The only value.
    #[default]
    CitationsDelta,
}

/// The `thinking_delta` literal.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum ThinkingDeltaType {
    /// The only value.
    #[default]
    ThinkingDelta,
}

/// The `signature_delta` literal.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum SignatureDeltaType {
    /// The only value.
    #[default]
    SignatureDelta,
}
