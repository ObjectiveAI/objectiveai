//! A listed provider's capacity, swapped and nothing else.

use serde::{Deserialize, Serialize};

use crate::provider::endpoints::capacity;

/// The provider's capacity changed, and nothing else of the item did:
/// the key the item was listed under, and the capacity as the daemon
/// cached it now. A client reads the item as last sent with this
/// `capacity` from now on; the item itself is not sent again for a
/// capacity alone, which changes far more often than anything else
/// in it.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[derive(schemars::JsonSchema)]
pub struct Swap {
    /// The provider's address, which is its identity: the key the
    /// item was listed under.
    pub address: String,
    /// The capacity cached now, which the item's `capacity` member
    /// is read as from now on.
    pub capacity: capacity::server::response::Frame,
}
