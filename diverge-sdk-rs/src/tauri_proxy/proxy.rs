//! What the app manages.

use crate::identities::Token;
use crate::wire::proxy::Link;

/// The state the commands read: one [`Link`] to one daemon. The app
/// manages exactly one, with `tauri::Builder::manage`; the commands
/// fail to resolve it otherwise.
pub struct TauriProxy(Link);

impl TauriProxy {
    /// The daemon at `address`, presenting `token`, not yet dialled:
    /// the page's first use dials it. An address with no scheme is
    /// dialled as `ws://<address>/`. The token is the app's identity's,
    /// from the host's identities.
    pub fn new(address: impl Into<String>, token: &Token) -> TauriProxy {
        TauriProxy(Link::new(address, token.expose()))
    }

    /// The link the commands use.
    pub fn link(&self) -> &Link {
        &self.0
    }
}
