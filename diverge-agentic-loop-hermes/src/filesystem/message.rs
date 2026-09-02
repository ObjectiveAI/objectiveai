//! One message of a session's transcript, as `/v1/runs` takes it.

use serde::Serialize;

/// One prior message, as `/v1/runs` takes history: a role and its
/// text. Nothing richer survives that field — the gateway reads
/// `role` and `content` as strings and nothing else — so this is
/// also all [`history`](super::history) reads out of the database.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Message {
    /// `user` or `assistant`, as the transcript spells them.
    pub role: String,
    /// The message text.
    pub content: String,
}
