//! The agent starting to run on a provider, as the log keeps it.

use serde::{Deserialize, Serialize};

use crate::shared::provider::Provider;

/// The agent became active: its container came up on the provider
/// named, and the agent is running there. `type` is the string
/// `active`, and the provider's members lie beside it.
///
/// Kept when the daemon has the agent running on a provider. Every
/// chunk that follows, until the [`Inactive`](super::Inactive) that
/// matches, was produced there. A run that never came up keeps an
/// [`Error`](super::Error) and no `active`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Active {
    /// Always `active`.
    pub r#type: ActiveType,
    /// The provider the agent is running on.
    #[serde(flatten)]
    pub provider: Provider,
}

/// [`Active`]'s discriminator.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum ActiveType {
    #[serde(rename = "active")]
    #[default]
    Active,
}
