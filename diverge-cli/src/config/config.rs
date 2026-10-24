//! The document.

use diverge_sdk::daemon::DEFAULT_PORT;
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use super::Daemon;

/// The name the built-in default daemon has.
pub const LOCAL: &str = "local";

/// The whole of `cli.yaml`: the daemons, by name, and which is dialled
/// when the command line names none.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct Config {
    /// The daemon dialled when the command line names none, by its
    /// name in `daemons`.
    pub default: String,
    /// The daemons the CLI may dial, by name, in the order the document
    /// gives them.
    pub daemons: IndexMap<String, Daemon>,
}

impl Default for Config {
    /// The built-in default: one daemon, [`LOCAL`], on this host at
    /// the daemon's own default port, with no account: a daemon admits
    /// no one it holds no account for, and the CLI holds none until the
    /// document names one.
    fn default() -> Self {
        let local = Daemon {
            address: format!("ws://127.0.0.1:{DEFAULT_PORT}/"),
            default: None,
            accounts: IndexMap::new(),
        };
        Config {
            default: LOCAL.to_string(),
            daemons: IndexMap::from([(LOCAL.to_string(), local)]),
        }
    }
}
