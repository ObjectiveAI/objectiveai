//! Why a command could not send, as the page reads it.

use std::fmt;

use serde::Serialize;

use crate::wire::client::handle::SendError;

/// What a command rejects with: a [`SendError`] in the two kinds the
/// page's own send error has. Serialized as
/// `{"kind": "connection", "message": "..."}` or
/// `{"kind": "scope", "message": "..."}`, which is exactly what the
/// TypeScript proxy handle reads back into its `SendError`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", content = "message", rename_all = "snake_case")]
pub enum Refusal {
    /// Nothing will go out on this connection: the socket is gone, the
    /// router is gone, nothing is dialled, or the dial itself failed.
    /// The message says which.
    Connection(String),
    /// This end has no such scope open. The connection is fine and
    /// every other scope on it is unaffected.
    Scope(String),
}

impl fmt::Display for Refusal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Refusal::Connection(message) | Refusal::Scope(message) => f.write_str(message),
        }
    }
}

impl std::error::Error for Refusal {}

/// The handle's error in the page's two kinds: a scope not open is
/// `scope`; the connection or the router gone is `connection`.
impl From<SendError> for Refusal {
    fn from(error: SendError) -> Self {
        match error {
            SendError::Scope => Refusal::Scope(error.to_string()),
            SendError::Connection(_) | SendError::Router => Refusal::Connection(error.to_string()),
        }
    }
}
