//! What the app manages.

use crate::keyring::Key;
use crate::wire::proxy::Link;

/// The state the commands read: one [`Link`] to one daemon. The app
/// manages exactly one, with `tauri::Builder::manage`; the commands
/// fail to resolve it otherwise.
pub struct TauriProxy(Link);

impl TauriProxy {
    /// The daemon at `address`, presenting `key`, not yet dialled: the
    /// page's first use dials it. An address with no scheme is dialled
    /// as `ws://<address>/`. The key is the app's, from its keyring.
    pub fn new(address: impl Into<String>, key: &Key) -> TauriProxy {
        TauriProxy(Link::new(address, key.expose()))
    }

    /// The link the commands use.
    pub fn link(&self) -> &Link {
        &self.0
    }
}
