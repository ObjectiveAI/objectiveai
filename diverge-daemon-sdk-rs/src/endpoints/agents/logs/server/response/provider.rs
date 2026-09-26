//! What the daemon knows about a provider.

use serde::{Deserialize, Serialize};

use super::Identity;

/// A provider, as the daemon knows it: who it is.
///
/// The provider protocol carries no provider identity on its wire —
/// no name, no key, no address type; a provider says which revision
/// of the specification it speaks and nothing else about itself. Who
/// a provider is is a fact of the CONNECTION, and [`Identity`] is
/// that fact. Flattened into the items that say where an agent ran,
/// [`Active`](super::Active) and [`Inactive`](super::Inactive), so
/// that its members lie beside the item's own. Today that is one
/// member, `identity`; what the daemon comes to know later, the
/// revision a provider answered on the protocol's `version` scope
/// say, lands beside it without moving anything.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Provider {
    /// Who the provider is. See [`Identity`].
    pub identity: Identity,
}
