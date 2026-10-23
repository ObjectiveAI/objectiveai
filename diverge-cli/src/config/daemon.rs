//! One daemon, as the document holds it.

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use super::Account;

/// The name a daemon's default account has when the document does
/// not say: the account a fresh daemon seeds.
pub const ROOT: &str = "root";

/// The keyring entry a fresh daemon seeds the root account's key in:
/// the default account's key, on the daemon's own host.
pub const ROOT_KEY: &str = "daemon/accounts/root";

/// A daemon the CLI may dial: where, and as whom.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Daemon {
    /// Where it is dialled: a WebSocket URL, or a host and port
    /// dialled as `ws://<address>/`.
    pub address: String,
    /// The account dialled as when the command line names none, by
    /// its name in `accounts`. `root` unless the document says.
    #[serde(default = "root")]
    pub default: String,
    /// The accounts the CLI may dial it as, by name, in the order the
    /// document gives them.
    #[serde(default)]
    pub accounts: IndexMap<String, Account>,
}

/// [`ROOT`], as a serde default.
fn root() -> String {
    ROOT.to_string()
}
