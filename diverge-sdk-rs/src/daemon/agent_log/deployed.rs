//! The agent's container up on a provider, as the log keeps it.

use serde::{Deserialize, Serialize};

use crate::daemon::lifecycle;

/// The agent's container came up on the provider named. `type` is the
/// string `deployed`, and the provider's members lie beside it — see
/// [`lifecycle::Deployed`]. A run that never came up keeps an
/// [`Error`](super::Error) and no `deployed`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[derive(schemars::JsonSchema)]
pub struct Deployed {
    /// Always `deployed`.
    pub r#type: DeployedType,
    /// The run's deployment, its members the item's.
    #[serde(flatten)]
    pub deployed: lifecycle::Deployed,
}

/// [`Deployed`]'s discriminator.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[derive(schemars::JsonSchema)]
pub enum DeployedType {
    #[serde(rename = "deployed")]
    #[default]
    Deployed,
}
