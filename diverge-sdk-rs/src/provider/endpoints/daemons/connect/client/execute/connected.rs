//! A daemon connection, open.

use std::fmt;

use tokio::sync::watch;

use crate::wire::client::handle::Handle;

/// A connection to another daemon, carried inside a connect scope: a
/// handle on it, which is a client of the daemon protocol like any on
/// a socket, and the connection's end.
///
/// # Dropping it ends it
///
/// The handle is the one writer of the connection's client frames;
/// every clone of it dropped is the frames channel finished, which is
/// this end hanging up. The acceptor hanging up, or the provider
/// ending the scope, is what [`ended`](Self::ended) resolves on, and
/// every request on the handle fails from then on.
pub struct Connected {
    /// The daemon reached, as a client reaches one.
    pub handle: Handle,
    ended: watch::Receiver<bool>,
}

impl Connected {
    pub(super) fn new(handle: Handle, ended: watch::Receiver<bool>) -> Self {
        Connected { handle, ended }
    }

    /// Resolves when the connection has ended: the acceptor hung up,
    /// the provider ended the scope, or the provider's connection
    /// went.
    pub async fn ended(&self) {
        let mut ended = self.ended.clone();
        while !*ended.borrow_and_update() {
            if ended.changed().await.is_err() {
                return;
            }
        }
    }

    /// Whether the connection has ended.
    pub fn is_ended(&self) -> bool {
        *self.ended.borrow()
    }
}

impl fmt::Debug for Connected {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Connected").field("ended", &self.is_ended()).finish_non_exhaustive()
    }
}
