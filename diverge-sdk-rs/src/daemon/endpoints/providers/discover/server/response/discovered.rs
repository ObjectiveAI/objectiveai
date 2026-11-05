//! One identity discovered, and the providers it was seen on.

use serde::{Deserialize, Serialize};

use crate::shared::Identity;
use crate::shared::discover::Description;
use crate::shared::listing::Keyed;

/// One identity discoverable through the daemon's providers, and every
/// connected provider it is discoverable on. The rules:
///
/// 1. **The identity.** `issuer` and `subject` are the `(iss, sub)`
///    under which the identity's connection to each listed provider was
///    authorized: the same on every provider listed, since that is what
///    makes it one item.
/// 2. **The providers.** Every connected provider whose own `discover`
///    listing holds the identity, by the provider's identity, in order —
///    by issuer, then by subject — never empty, and never repeating one.
/// 3. **The descriptions.** Every description the identity is listed
///    with on those providers, each once: descriptions that are equal
///    are one entry, however many providers list it. In order, and
///    empty when no provider lists it with one.
/// 4. **What it makes.** A [`daemons`](crate::daemon::endpoints::daemons)
///    record of it names `issuer` and `subject` as its `daemon`: the
///    remote it reaches, through any one of `providers`.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[derive(schemars::JsonSchema)]
pub struct Discovered {
    /// The `iss` claim the identity's connections were authorized under.
    pub issuer: String,
    /// The `sub` claim.
    pub subject: String,
    /// Every connected provider it is discoverable on, in order.
    pub providers: Vec<Identity>,
    /// Every distinct description it is listed with, in order.
    pub descriptions: Vec<Description>,
}

impl Discovered {
    /// The identity alone.
    pub fn identity(&self) -> Identity {
        Identity { issuer: self.issuer.clone(), subject: self.subject.clone() }
    }
}

/// Keyed by the identity.
impl Keyed for Discovered {
    type Key = Identity;

    fn key(&self) -> Identity {
        self.identity()
    }
}
