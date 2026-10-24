//! The `daemons` section of `daemon.yaml`.

use diverge_sdk::daemon::endpoints::providers::daemons::Link;
use diverge_sdk::shared::Identity;
use serde::{Deserialize, Serialize};

/// The other daemons this daemon reaches: the identities it accepts
/// from them, and how it reaches them. A daemon that reaches THIS one
/// is judged by no list here: it must be an account of this daemon's,
/// one daemon connecting as it at a time.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct Daemons {
    /// The identities a daemon this daemon reaches may answer as. A
    /// reached daemon whose verified `(issuer, subject)` is not here, or
    /// is held by another connection, is closed in the handshake.
    /// Absent, none.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub accepted: Vec<Identity>,
    /// The daemons this daemon reaches: see [`Remote`]. Absent, none.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub connect: Vec<Remote>,
}

/// Another daemon this daemon reaches, through a provider both are
/// connected to: what a connected tool and a relayed request name, by
/// `name`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Remote {
    /// The name the daemon is reached by here, unique among them.
    pub name: String,
    /// The issuer this daemon presents its token from: `identities`
    /// names its subject there. The remote answers from the same
    /// issuer, and its identity must be one `accepted` lists.
    pub issuer: String,
    /// The providers the remote is reached through, each with the
    /// identity the remote accepts there under: what the provider is
    /// asked to connect to.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub links: Vec<Link>,
}
