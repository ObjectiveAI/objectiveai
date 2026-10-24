//! What the app manages.

use std::sync::Arc;

use crate::wire::handshake::Authenticator;
use crate::wire::proxy::Link;

/// The state the commands read: one [`Link`] to one daemon. The app
/// manages exactly one, with `tauri::Builder::manage`; the commands
/// fail to resolve it otherwise.
pub struct TauriProxy(Link);

impl TauriProxy {
    /// The daemon at `address`, not yet dialled: the page's first use
    /// dials it, its handshake made from `issuer` through
    /// `authenticator` — the app's identity at that issuer, from the
    /// host's [`identities`](crate::identities), and its judgment of
    /// the daemon's. An address with no scheme is dialled as
    /// `ws://<address>/`.
    pub fn new(address: impl Into<String>, issuer: impl Into<String>, authenticator: Arc<dyn Authenticator>) -> TauriProxy {
        TauriProxy(Link::new(address, issuer, authenticator))
    }

    /// The link the commands use.
    pub fn link(&self) -> &Link {
        &self.0
    }
}
