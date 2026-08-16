//! A connector that has left.

use serde::{Deserialize, Serialize};

/// One connector, gone.
///
/// An object rather than a bare nickname, so a provider that later has
/// more to say about a departure — when, or why — has somewhere to say
/// it without changing what a nickname is.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub struct Disconnected {
    /// The name the runner gave this connector when it
    /// [`Authorized`](crate::endpoints::laboratories::run::client::channel_response::authorize::Frame::Authorized)
    /// it.
    ///
    /// Absent when the runner gave none, which is a runner that did
    /// not care to tell its connectors apart. Nothing downstream has
    /// to invent an identity nobody supplied.
    ///
    /// The provider stores it and hands it back; it never read it, and
    /// the connector was never told it had one.
    ///
    /// Nothing requires it to be unique. Two connectors may share a
    /// nickname, and a departure then names both and resolves neither
    /// — which is the runner's own arrangement rather than something
    /// this protocol did to it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub nickname: Option<String>,
}
