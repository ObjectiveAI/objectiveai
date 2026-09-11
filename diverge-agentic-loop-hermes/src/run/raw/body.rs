//! What actually goes on the wire: the request, plus the history.

use serde::Serialize;

use super::Request;
use crate::filesystem::Message;

/// The `POST /v1/runs` body: the caller's [`Request`] and, when it
/// names a session, that session's transcript as the gateway takes
/// it.
#[derive(Debug, Serialize)]
pub(super) struct Body<'a> {
    /// The caller's fields, flattened alongside.
    #[serde(flatten)]
    pub request: &'a Request,
    /// The conversation so far — read by
    /// [`history`](crate::filesystem::history) for the named session.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub conversation_history: Option<Vec<Message>>,
}
