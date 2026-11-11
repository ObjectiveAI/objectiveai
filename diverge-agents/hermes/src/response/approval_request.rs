//! The `approval.request` event.

use serde::Deserialize;

/// The agent is waiting on a permission decision — answered out of
/// band at `POST /v1/runs/{id}/approval`, never on this stream.
///
/// The one OPEN payload in the vocabulary: the gateway forwards
/// the firing guard's whole dict unfiltered, overwriting only the
/// trio and [`choices`](Self::choices) — so beside the keys known
/// at the pin, [`extra`](Self::extra) catches whatever a guard or
/// plugin adds tomorrow, and nothing on this event can fail to
/// read.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct ApprovalRequest {
    /// The discriminator. Always `approval.request`.
    pub event: ApprovalRequestEvent,
    /// The run.
    pub run_id: String,
    /// Seconds since the epoch, fractional.
    pub timestamp: f64,
    /// The answers the gateway will accept, drawn from `once`,
    /// `session`, `always`, `deny`.
    pub choices: Vec<String>,
    /// The entry's id, injected by the approval store.
    #[serde(default)]
    pub request_id: Option<String>,
    /// The command awaiting judgment, redacted upstream.
    #[serde(default)]
    pub command: Option<String>,
    /// What the guard says it is.
    #[serde(default)]
    pub description: Option<String>,
    /// The pattern a `session`/`always` answer would remember.
    #[serde(default)]
    pub pattern_key: Option<String>,
    /// Several patterns, where the guard offers them.
    #[serde(default)]
    pub pattern_keys: Option<Vec<String>>,
    /// Whether `always` is on the table.
    #[serde(default)]
    pub allow_permanent: Option<bool>,
    /// Whether `session` is on the table.
    #[serde(default)]
    pub allow_session: Option<bool>,
    /// Present (true) when a smart-deny already judged this.
    #[serde(default)]
    pub smart_denied: Option<bool>,
    /// Everything else the guard put in its payload.
    #[serde(flatten)]
    pub extra: serde_json::Map<String, serde_json::Value>,
}

/// [`ApprovalRequest`]'s discriminator: the one value no other
/// event carries, which is what lets the union stay untagged.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize)]
pub enum ApprovalRequestEvent {
    /// The only value.
    #[serde(rename = "approval.request")]
    ApprovalRequest,
}
