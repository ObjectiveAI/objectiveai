//! The agent ceasing to run on a provider, as the log keeps it.

use serde::{Deserialize, Serialize};

use super::Provider;

/// The agent became inactive: it no longer runs on the provider
/// named. `type` is the string `inactive`, and the provider's members
/// lie beside it.
///
/// Kept when the agent ceases to run on a provider, on any ending —
/// the run's own finish, the daemon's stop, the provider's loss. The
/// provider is the one the last [`Active`](super::Active) named, so
/// the two bracket every chunk produced there; nothing between an
/// `inactive` and the next `active` was said by the agent.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Inactive {
    /// Always `inactive`.
    pub r#type: InactiveType,
    /// The provider the agent ran on.
    #[serde(flatten)]
    pub provider: Provider,
}

/// [`Inactive`]'s discriminator.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum InactiveType {
    #[serde(rename = "inactive")]
    #[default]
    Inactive,
}
