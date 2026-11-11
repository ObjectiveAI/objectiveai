//! The agent beginning to work, as the log keeps it.

use serde::{Deserialize, Serialize};

/// The agent began working: a loop runs in it, or an MCP exchange is
/// in flight for it — its container's own, a dependency's, or one run
/// as it through `agents connect`. `type` is the string `active`, and
/// nothing lies beside it.
///
/// Kept only between the run's [`Deployed`](super::Deployed) and its
/// [`Undeployed`](super::Undeployed), and only on a change — see
/// [`lifecycle`](crate::daemon::lifecycle). Every chunk the agent
/// said falls inside a run's `deployed` … `undeployed`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[derive(schemars::JsonSchema)]
pub struct Active {
    /// Always `active`.
    pub r#type: ActiveType,
}

/// [`Active`]'s discriminator.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[derive(schemars::JsonSchema)]
pub enum ActiveType {
    #[serde(rename = "active")]
    #[default]
    Active,
}
