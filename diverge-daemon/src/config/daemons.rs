//! The `daemons` section of `daemon.yaml`.

use diverge_sdk::daemon::endpoints::providers::daemons::Link;
use serde::{Deserialize, Serialize};

/// Another daemon this daemon reaches, through a provider both are
/// connected to: what a connected tool and a relayed request name, by
/// `name`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Remote {
    /// The name the daemon is reached by here, unique among them.
    pub name: String,
    /// The issuer this daemon presents its token from: `identities`
    /// names its subject there, and the remote holds that identity as
    /// one of its accounts.
    pub issuer: String,
    /// The providers the remote is reached through, each with the
    /// identity the remote accepts there under.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub links: Vec<Link>,
}
