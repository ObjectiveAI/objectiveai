//! One tool, once and for all.

use serde::{Deserialize, Serialize};

use super::Origin;

/// One tool of the caller's, by its fixed [`Origin`] and its index,
/// with its name beside when it has one.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Tool {
    /// What it was made with: see [`Origin`].
    pub origin: Origin,
    /// Its number among all tools of the caller's ever made with that
    /// origin, as its list item carries it.
    pub index: u64,
    /// Its name, as its create or its connect gave it, if it gave
    /// one. Absent when it has none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}
