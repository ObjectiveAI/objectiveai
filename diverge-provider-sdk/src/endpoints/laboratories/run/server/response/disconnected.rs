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
    /// Always present, because an authorization always carried one. A
    /// runner that did not care to tell its connectors apart gave them
    /// all the same name — most likely the empty string — and their
    /// departures arrive under it, distinguishing nothing. Which is
    /// what a missing name said, without a second case to hold.
    ///
    /// The provider stores it and hands it back; it never read it, and
    /// the connector was never told it had one.
    ///
    /// Nothing requires it to be unique. Two connectors may share a
    /// nickname, and a departure then names both and resolves neither
    /// — which is the runner's own arrangement rather than something
    /// this protocol did to it.
    pub nickname: String,
}
