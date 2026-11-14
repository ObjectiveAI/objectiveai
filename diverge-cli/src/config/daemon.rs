//! One daemon, as the document holds it.

use diverge_sdk::shared::Identity;
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use super::Account;

/// A daemon the CLI may dial: where, and as whom.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Daemon {
    /// Where it is dialled: a WebSocket URL, or a host and port
    /// dialled as `ws://<address>/`.
    pub address: String,
    /// The account dialled as when the command line names none, by
    /// its name in `accounts`. Absent, a run must name one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default: Option<String>,
    /// The daemon's own identity, `{issuer, subject}`: who must answer
    /// at the address. Stated, the handshake fails unless that
    /// identity answers and proves the key its token names, so the
    /// CLI reached the daemon it meant. Absent, whichever identity
    /// answers with a token that verifies is taken: the CLI then
    /// knows it reached someone who is who they say, and not whom.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub identity: Option<Identity>,
    /// The accounts the CLI may dial it as, by name, in the order the
    /// document gives them.
    #[serde(default)]
    pub accounts: IndexMap<String, Account>,
}
