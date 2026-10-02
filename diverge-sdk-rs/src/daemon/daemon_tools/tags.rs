//! The daemon's tools that tag and untag, and how far they reach.

use serde::{Deserialize, Serialize};

/// How far an agent's, or a tool's, tagging reaches: the one way there is for
/// now, with more to come. Snake case on the wire: `"free"`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Tags {
    /// It tags and untags freely: any tag, on anything of the
    /// caller's the tools reach.
    Free,
}
