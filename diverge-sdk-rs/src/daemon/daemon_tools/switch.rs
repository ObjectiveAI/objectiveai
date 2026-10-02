//! Whether a container holds one of the daemon's tools that has no
//! subject to narrow.

use serde::{Deserialize, Serialize};

/// The reach of a tool that makes something from nothing — a
/// template create, a resource upload — which has nothing to narrow:
/// the container holds it, or does not. Externally tagged JSON: the
/// string `"disabled"` or the string `"any"`, the two variants of
/// [`Reach`](super::Reach) that need no `T`. `Disabled` is the
/// default, and what a member left out decodes as.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Switch {
    /// The container does not hold the tool.
    #[default]
    Disabled,
    /// The container holds the tool.
    Any,
}
