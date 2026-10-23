//! The daemon the viewer dials.

use diverge_sdk::daemon::DEFAULT_PORT;
use serde::{Deserialize, Serialize};

/// The keyring entry the daemon's root account's key is kept in on
/// the daemon's own host, which every daemon seeds, as the CLI's own
/// default names it.
pub const ROOT: &str = "daemon/accounts/root";

/// The daemon the viewer dials, and as whom: the address, and the
/// keyring entry of the key of the account the connection is
/// presented as. The document holds no key: the app reads it from the
/// root's keyring at start, and it never reaches the page, which
/// speaks to the daemon only through the app's own Rust.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Daemon {
    /// Where the daemon is dialled: a WebSocket URL, or a bare
    /// `host:port` the viewer puts `ws://` before and `/` after.
    pub address: String,
    /// The name of the entry in the root's keyring whose key the
    /// connection presents: the entry an account's create named, or
    /// `daemon/accounts/root` for the root account.
    pub key: String,
}

/// The daemon on this host, as its root account.
impl Default for Daemon {
    fn default() -> Self {
        Daemon { address: format!("ws://127.0.0.1:{DEFAULT_PORT}/"), key: ROOT.to_string() }
    }
}
