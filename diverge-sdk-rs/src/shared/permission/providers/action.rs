//! What may be done to providers.

use serde::{Deserialize, Serialize};

/// An act over the daemon's providers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[derive(schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Action {
    /// Read one, by identity.
    Get,
    /// List them.
    List,
    /// Discover the identities their discover listings hold, as
    /// [`providers::discover`](crate::daemon::endpoints::providers::discover)
    /// does.
    Discover,
    /// Add one.
    Add,
    /// Edit one.
    Edit,
    /// Delete one.
    Delete,
    /// Enable one.
    Enable,
    /// Disable one: its runs taken down.
    Disable,
    /// Put tags on one.
    Tag,
    /// Take tags off one.
    Untag,
}
