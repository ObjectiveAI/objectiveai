//! One agent, once and for all.

use serde::{Deserialize, Serialize};

/// One agent of the caller's, by its template and its index, with
/// its name beside when it has one.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Agent {
    /// The template it was made from, by id.
    pub template: String,
    /// Its number among all agents of the caller's ever made from
    /// that template, as its list item carries it.
    pub index: u64,
    /// Its name, as its create gave it, if it gave one. Absent when
    /// it has none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}
