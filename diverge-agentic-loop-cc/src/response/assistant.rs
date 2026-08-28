//! The `assistant` records: what the model said.

use serde::{Deserialize, Serialize};

use super::message;

/// A `type: "assistant"` record: one content block of an assistant
/// message, wrapped with the run's own bookkeeping.
///
/// One BLOCK, not one message: Claude Code splits every assistant
/// message into a record per block before writing, so
/// [`message`](Self::message)'s content is a one-element array on the
/// wire and the original message reassembles by grouping on the
/// message's own `id`. The record [`uuid`](Self::uuid)s are derived
/// per block and do not group anything.
///
/// # The subagent marker
///
/// A record whose [`parent_tool_use_id`](Self::parent_tool_use_id)
/// is non-null came from a subagent, keyed to the tool call that
/// spawned it. There is no other marker — the transcript's
/// `isSidechain` never reaches stdout.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Assistant {
    /// Always `assistant`.
    pub r#type: AssistantType,
    /// The API message, carrying this record's one block.
    pub message: message::Message,
    /// The spawning tool call, for a subagent's record; `null` on
    /// the main thread.
    pub parent_tool_use_id: Option<String>,
    /// Set when the message is an API failure standing in for an
    /// answer.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<AssistantMessageError>,
    /// The record's own id — per BLOCK, see the type doc.
    pub uuid: String,
    /// The session.
    pub session_id: String,
}

/// The kinds of API failure an assistant record can stand for — the
/// same vocabulary an
/// [`api_retry`](super::system::System::ApiRetry) record uses.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum AssistantMessageError {
    /// The credential did not work.
    AuthenticationFailed,
    /// Billing said no.
    BillingError,
    /// Rate limited.
    RateLimit,
    /// The request itself was rejected.
    InvalidRequest,
    /// The server broke.
    ServerError,
    /// Something else.
    Unknown,
    /// The output token ceiling.
    MaxOutputTokens,
}

/// The `assistant` literal.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum AssistantType {
    /// The only value.
    #[default]
    Assistant,
}
