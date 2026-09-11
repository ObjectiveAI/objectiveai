//! One event of the run stream.

use serde::Deserialize;

/// Everything `GET /v1/runs/{id}/events` can carry, one frame at a
/// time.
///
/// Untagged, discriminated by each payload's own `event` constant —
/// the gateway writes `data:`-only frames, never an SSE `event:`
/// line, so the JSON body is the whole of the discrimination. The
/// twelve typed variants are the pinned gateway's complete
/// vocabulary; [`Unknown`](Self::Unknown) is LAST and catches any
/// JSON at all, so a Hermes newer than the pin still parses and
/// the harness drops or relays what it does not know KNOWINGLY
/// rather than dying on it.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(untagged)]
pub enum Event {
    /// See [`ToolStarted`](super::ToolStarted).
    ToolStarted(super::ToolStarted),
    /// See [`ToolCompleted`](super::ToolCompleted).
    ToolCompleted(super::ToolCompleted),
    /// See [`ReasoningAvailable`](super::ReasoningAvailable).
    ReasoningAvailable(super::ReasoningAvailable),
    /// See [`SubagentStart`](super::SubagentStart).
    SubagentStart(super::SubagentStart),
    /// See [`SubagentComplete`](super::SubagentComplete).
    SubagentComplete(super::SubagentComplete),
    /// See [`MessageDelta`](super::MessageDelta).
    MessageDelta(super::MessageDelta),
    /// See [`ApprovalRequest`](super::ApprovalRequest).
    ApprovalRequest(super::ApprovalRequest),
    /// See [`ApprovalResponded`](super::ApprovalResponded).
    ApprovalResponded(super::ApprovalResponded),
    /// See [`RunSteered`](super::RunSteered).
    RunSteered(super::RunSteered),
    /// See [`RunCancelled`](super::RunCancelled).
    RunCancelled(super::RunCancelled),
    /// See [`RunFailed`](super::RunFailed).
    RunFailed(super::RunFailed),
    /// See [`RunCompleted`](super::RunCompleted).
    RunCompleted(super::RunCompleted),
    /// Anything else — a frame from a gateway newer than the pin.
    /// Kept whole so nothing on the stream is ever unreadable.
    Unknown(serde_json::Value),
}
