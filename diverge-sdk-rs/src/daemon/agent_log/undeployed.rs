//! The agent's container down, as the log keeps it.

use serde::{Deserialize, Serialize};

use crate::daemon::lifecycle;

/// The agent's container went down, on any ending. `type` is the
/// string `undeployed`, and the provider's members, the `reason` when
/// the end was unexpected and `interrupted` lie beside it — see
/// [`lifecycle::Undeployed`]. Nothing between an `undeployed` and the
/// next [`Deployed`](super::Deployed) was said by the agent.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[derive(schemars::JsonSchema)]
pub struct Undeployed {
    /// Always `undeployed`.
    pub r#type: UndeployedType,
    /// The run's end, its members the item's.
    #[serde(flatten)]
    pub undeployed: lifecycle::Undeployed,
}

/// [`Undeployed`]'s discriminator.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[derive(schemars::JsonSchema)]
pub enum UndeployedType {
    #[serde(rename = "undeployed")]
    #[default]
    Undeployed,
}
