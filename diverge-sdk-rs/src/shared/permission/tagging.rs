//! The two tagging actions.

use serde::{Deserialize, Serialize};

/// Putting tags on, or taking them off: the actions of a tagging grant,
/// the same two for every kind that carries tags. Snake case on the
/// wire: `"tag"`, `"untag"`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[derive(schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Tagging {
    /// Put tags on, as the kind's `tag` endpoint does.
    Tag,
    /// Take tags off, as the kind's `untag` endpoint does.
    Untag,
}
