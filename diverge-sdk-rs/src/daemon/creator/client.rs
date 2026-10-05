//! The client as a creator.

use serde::{Deserialize, Serialize};

/// The client itself: what made a thing over an endpoint, and what
/// every agent and tool of the client's descends from, one maker at
/// a time.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Client {
    /// The account the create was served for, as
    /// [`accounts`](crate::daemon::endpoints::accounts) states the
    /// identity: the account's name when it has one, and otherwise the
    /// identity its credential names. The daemon's word for a caller,
    /// compared and not read.
    pub identity: String,
}
