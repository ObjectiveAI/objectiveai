//! What the daemon knows about a provider.

use serde::{Deserialize, Serialize};

use super::Identity;

/// A provider, as the daemon knows it: who it is.
///
/// Flattened into whatever names a provider — a log item that says
/// where an agent ran — so that its members lie beside the item's
/// own. Today that is one member, `identity`; what the daemon comes
/// to know later, the revision a provider answered on the protocol's
/// `version` scope say, lands beside it without moving anything.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Provider {
    /// Who the provider is. See [`Identity`].
    pub identity: Identity,
}
