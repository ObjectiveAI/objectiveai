//! The `stream_event` records: the API's stream, relayed raw.

use serde::Deserialize;

use super::message;

/// A `type: "stream_event"` record: one raw API streaming event,
/// wrapped with the run's bookkeeping. Emitted only when the run was
/// started with `--include-partial-messages`; without the flag,
/// assistant content arrives only as whole-block records.
///
/// # The wrapper promises less than its siblings
///
/// [`parent_tool_use_id`](Self::parent_tool_use_id) is hardcoded
/// `null` at the source — a subagent's stream events are not
/// distinguishable here — and [`uuid`](Self::uuid) is freshly minted
/// per event, related to nothing.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct StreamEvent {
    /// Always `stream_event`.
    pub r#type: StreamEventType,
    /// The raw API event.
    pub event: message::StreamEvent,
    /// Always `null` in the pinned source; see the type doc.
    pub parent_tool_use_id: Option<String>,
    /// A fresh id per event; groups nothing.
    pub uuid: String,
    /// The session.
    pub session_id: String,
}

/// The `stream_event` literal.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum StreamEventType {
    /// The only value.
    #[default]
    StreamEvent,
}
