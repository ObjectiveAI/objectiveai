//! One discoverable connected identity, and what it says of itself.

use serde::{Deserialize, Serialize};

use crate::shared::Identity;
use crate::shared::discover::Description;
use crate::shared::listing::Keyed;

/// An identity connected to the provider whose discover flag was `1`
/// or `2`, and the description its authorization frame carried: none
/// under the flag `1`. The description is the connection's, so it does
/// not change while the identity is listed.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct Discoverable {
    /// The identity, `(iss, sub)`.
    pub identity: Identity,
    /// What it says of itself, if anything.
    pub description: Option<Description>,
}

/// Keyed by the identity: an identity holds at most one connection to
/// a provider, so it is listed at most once.
impl Keyed for Discoverable {
    type Key = Identity;

    fn key(&self) -> Identity {
        self.identity.clone()
    }
}
