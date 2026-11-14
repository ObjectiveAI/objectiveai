//! The agent ceasing to work, as the log keeps it.

use serde::{Deserialize, Serialize};

/// The agent ceased working: no loop runs in it and no exchange is in
/// flight for it. `type` is the string `inactive`, and nothing lies
/// beside it.
///
/// Kept only after an [`Active`](super::Active) of the same run, and
/// never after the run's [`Undeployed`](super::Undeployed): a run
/// taken down while working keeps this first, then `undeployed` — see
/// [`lifecycle`](crate::daemon::lifecycle). A run going down while it
/// works keeps its last `inactive` just before `undeployed`,
/// interrupted. The agent's idle clock runs from here.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[derive(schemars::JsonSchema)]
pub struct Inactive {
    /// Always `inactive`.
    pub r#type: InactiveType,
}

/// [`Inactive`]'s discriminator.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[derive(schemars::JsonSchema)]
pub enum InactiveType {
    #[serde(rename = "inactive")]
    #[default]
    Inactive,
}
