//! The client as a creator.

use serde::{Deserialize, Serialize};

use crate::shared::Identity;

/// The client itself: what made a thing over an endpoint, and what
/// every agent and tool of the client's descends from, one maker at
/// a time.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[derive(schemars::JsonSchema)]
pub struct Client {
    /// The account the create was served for: its identity, see
    /// [`accounts`](crate::daemon::endpoints::accounts).
    pub identity: Identity,
}
