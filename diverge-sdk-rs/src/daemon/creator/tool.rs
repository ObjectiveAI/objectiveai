//! A tool as a creator.

use serde::{Deserialize, Serialize};

/// A tool of the client's that made something — one the client made
/// from a template, since a connected tool makes nothing: named once
/// and for all by its template and its index, and by its name as it
/// was called.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Tool {
    /// The template the tool was made from, by id.
    pub template: String,
    /// The tool's number among all tools of the client's ever made
    /// from that template: the
    /// [`index`](crate::daemon::endpoints::tools::list::server::response::Tool::index)
    /// its list item carries. The template and the index name the
    /// tool once and for all.
    pub index: u64,
    /// The tool's name, as its create gave it.
    pub name: String,
}
