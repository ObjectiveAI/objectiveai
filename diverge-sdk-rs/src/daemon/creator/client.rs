//! The client as a creator.

use serde::{Deserialize, Serialize};

/// The client itself: what made a thing over an endpoint, and what
/// every agent and tool of the client's descends from, one maker at
/// a time.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Client {
    /// The client's identity, as the daemon holds the connection the
    /// create arrived on. The daemon's word for a caller, compared
    /// and not read.
    pub identity: String,
}
