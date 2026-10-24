//! The `providers` section of `daemon.yaml`.

use diverge_sdk::shared::Identity;
use serde::{Deserialize, Serialize};

/// The providers the daemon works with: the identities it accepts, and
/// the addresses it dials. There is no incoming and no outgoing: a
/// provider is served the same whichever end dialled, by its identity.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct Providers {
    /// The identities of the providers the daemon works with. A
    /// provider whose verified `(issuer, subject)` is not here is
    /// closed in the handshake — one that dialled the daemon and one
    /// the daemon dialled alike. Absent, none.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub accepted: Vec<Identity>,
    /// The providers the daemon dials, redialling whenever the
    /// connection ends: see [`Connect`]. Absent, it dials none.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub connect: Vec<Connect>,
}

/// A provider the daemon dials.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Connect {
    /// Where the provider listens: `host:port`, dialled as
    /// `ws://<address>/`, or a URL with its own scheme.
    pub address: String,
    /// The issuer the daemon presents its token from: `identities`
    /// names the daemon's subject there. The provider answers from the
    /// same issuer, and its identity must be one `accepted` lists.
    pub issuer: String,
}
