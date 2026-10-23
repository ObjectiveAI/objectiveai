//! The daemon the viewer dials.

use diverge_sdk::daemon::DEFAULT_PORT;
use serde::{Deserialize, Serialize};

/// The key the daemon's root account is dialled with, which every
/// daemon has: the word `root`, as the CLI's own default names it.
pub const ROOT: &str = "root";

/// The daemon the viewer dials, and as whom: the address, and the key
/// of the account the connection is presented as. The key is the
/// credential itself, as `cli.yaml` holds one; it never reaches the
/// page, which speaks to the daemon only through the app's own Rust.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Daemon {
    /// Where the daemon is dialled: a WebSocket URL, or a bare
    /// `host:port` the viewer puts `ws://` before and `/` after.
    pub address: String,
    /// The key the connection presents: an account's, as the daemon
    /// minted it, or `root` for the root account.
    pub key: String,
}

/// The daemon on this host, as its root account.
impl Default for Daemon {
    fn default() -> Self {
        Daemon { address: format!("ws://127.0.0.1:{DEFAULT_PORT}/"), key: ROOT.to_string() }
    }
}
