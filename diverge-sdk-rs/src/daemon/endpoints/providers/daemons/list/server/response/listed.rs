//! One identity listed, and the routes it was seen on.

use serde::{Deserialize, Serialize};

use super::Via;
use crate::shared::Identity;
use crate::shared::description::Description;
use crate::shared::listing::Keyed;

/// One identity listable through the daemon's providers, and every
/// route to it: each connected provider it is listable on, with
/// where. The rules:
///
/// 1. **The identity.** `issuer` and `subject` are the `(iss, sub)`
///    under which the identity's connection to each listed provider was
///    authorized: the same on every provider listed, since that is what
///    makes it one item.
/// 2. **The routes.** One [`Via`] per connected provider whose own
///    `daemons::list` holds the identity: the provider, the IP of the
///    identity's connection to it, and the dial target when the provider
///    dialled it. In order of provider — by issuer, then by subject —
///    never empty, and never repeating a provider, since an identity
///    holds one connection to a provider.
/// 3. **The descriptions.** Every description the identity is listed
///    with on those providers, each once: descriptions that are equal
///    are one entry, however many providers list it. In order, and
///    empty when no provider lists it with one.
/// 4. **What it makes.** A [`daemons`](crate::daemon::endpoints::daemons)
///    record of it names `issuer` and `subject` as its `daemon`: the
///    remote it reaches, through any one of `providers` its filters
///    let through.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[derive(schemars::JsonSchema)]
pub struct Listed {
    /// The `iss` claim the identity's connections were authorized under.
    pub issuer: String,
    /// The `sub` claim.
    pub subject: String,
    /// Every route to it, in order of provider.
    pub providers: Vec<Via>,
    /// Every distinct description it is listed with, in order.
    pub descriptions: Vec<Description>,
}

impl Listed {
    /// The identity alone.
    pub fn identity(&self) -> Identity {
        Identity { issuer: self.issuer.clone(), subject: self.subject.clone() }
    }
}

/// Keyed by the identity.
impl Keyed for Listed {
    type Key = Identity;

    fn key(&self) -> Identity {
        self.identity()
    }
}
