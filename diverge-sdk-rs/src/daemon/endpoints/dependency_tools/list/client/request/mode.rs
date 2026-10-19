//! Which of its two modes a dependency template has.

use serde::{Deserialize, Serialize};

/// Transient or a service: the `mode` a dependency template's
/// [`Mode`](crate::shared::containers::dependencies::Mode) is tagged
/// with, as a value of its own to narrow a list by. Snake case on the
/// wire, as the tag is: `"transient"`, `"service"`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[derive(schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Mode {
    /// A template deployed for each declaring agent's run, and ended
    /// with it.
    Transient,
    /// A template kept as a service under the scope its `scope`
    /// names, for as long as any agent is attached to it.
    Service,
}
