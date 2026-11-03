//! The `providers` section of `daemon.yaml`.

use diverge_sdk::shared::Identity;
use serde::{Deserialize, Serialize};

/// The providers the daemon works with: the identities it accepts, and
/// the addresses it dials. There is no incoming and no outgoing: a
/// provider is served the same whichever end dialled, by its identity.
///
/// # The discover flag
///
/// Every handshake with a provider carries the daemon's discover flag:
/// whether the provider lists the daemon to its `discover`. It is set
/// per provider, and `false` unless set. The rules:
///
/// 1. **A provider the daemon dials.** The flag is its [`Connect`]'s
///    `discover`, since the daemon sends it before it knows which
///    provider answers.
/// 2. **A provider that dials the daemon.** The flag is its
///    [`Accepted`]'s `discover`.
/// 3. **Everyone else.** A client, and a daemon reaching this one
///    through a provider, receives the flag `false`, and ignores it.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct Providers {
    /// The identities of the providers the daemon works with. A
    /// provider whose verified `(issuer, subject)` is not here is
    /// closed in the handshake — one that dialled the daemon and one
    /// the daemon dialled alike. Absent, none.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub accepted: Vec<Accepted>,
    /// The providers the daemon dials, redialling whenever the
    /// connection ends: see [`Connect`]. Absent, it dials none.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub connect: Vec<Connect>,
}

impl Providers {
    /// The accepted providers' identities, in the configuration's
    /// order.
    pub fn identities(&self) -> Vec<Identity> {
        self.accepted.iter().map(Accepted::identity).collect()
    }

    /// Whether `identity` is an accepted provider's.
    pub fn accepts(&self, identity: &Identity) -> bool {
        self.accepted.iter().any(|accepted| accepted.issuer == identity.issuer && accepted.subject == identity.subject)
    }

    /// The identities of the accepted providers marked `discover`: the
    /// providers that dial in which list the daemon — see rule 2.
    pub fn discoverable(&self) -> Vec<Identity> {
        self.accepted.iter().filter(|accepted| accepted.discover).map(Accepted::identity).collect()
    }
}

/// A provider the daemon works with, by its identity.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Accepted {
    /// The provider's issuer.
    pub issuer: String,
    /// The provider's subject at that issuer.
    pub subject: String,
    /// Whether the daemon is discoverable to this provider when the
    /// provider dials it — see rule 2 of [`Providers`]. Absent, `false`.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub discover: bool,
}

impl Accepted {
    /// The provider's identity, `(issuer, subject)`.
    pub fn identity(&self) -> Identity {
        Identity { issuer: self.issuer.clone(), subject: self.subject.clone() }
    }
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
    /// Whether the daemon is discoverable to this provider — see rule 1
    /// of [`Providers`]. Absent, `false`.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub discover: bool,
}
