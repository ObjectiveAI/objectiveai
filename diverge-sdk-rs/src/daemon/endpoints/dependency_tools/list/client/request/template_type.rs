//! Which of its two types a dependency template has.

use serde::{Deserialize, Serialize};

/// Transient or a service: the `type` a dependency template's
/// [`Kind`](crate::shared::containers::dependencies::Kind) is tagged
/// with, as a value of its own to narrow a list by. Snake case on the
/// wire, as the tag is: `"transient"`, `"service"`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Type {
    /// A template deployed for each declaring agent's run, and ended
    /// with it.
    Transient,
    /// A template kept as a service under the scope its `scope`
    /// names, for as long as any agent is attached to it.
    Service,
}
