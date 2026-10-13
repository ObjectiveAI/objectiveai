//! The document.

use std::collections::BTreeMap;

use diverge_sdk::config::daemon;
use serde::{Deserialize, Serialize};

use super::{Account, Daemon, ROOT};

/// The name the built-in default daemon has.
pub const LOCAL: &str = "local";

/// The whole of `cli.json`: the daemons, by name, and which is dialled
/// when the command line names none.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct Config {
    /// The daemon dialled when the command line names none, by its
    /// name in `daemons`.
    pub default: String,
    /// The daemons the CLI may dial, by name.
    pub daemons: BTreeMap<String, Daemon>,
}

impl Default for Config {
    /// The built-in default: one daemon, [`LOCAL`], on this host at
    /// the daemon's own default port, holding the one account
    /// [`ROOT`] with the key a fresh daemon seeds, which is the word
    /// `root`.
    fn default() -> Self {
        let port = daemon::Config::default().port;
        let local = Daemon {
            address: format!("ws://127.0.0.1:{port}/"),
            default: ROOT.to_string(),
            accounts: BTreeMap::from([(
                ROOT.to_string(),
                Account {
                    key: ROOT.to_string(),
                },
            )]),
        };
        Config {
            default: LOCAL.to_string(),
            daemons: BTreeMap::from([(LOCAL.to_string(), local)]),
        }
    }
}
