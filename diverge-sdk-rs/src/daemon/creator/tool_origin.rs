//! What a tool was made with, as much of it as never changes.

use serde::{Deserialize, Serialize};

use crate::daemon::endpoints::agents::logs::server::response::Identity;

/// The fixed part of a tool's
/// [`Origin`](crate::daemon::endpoints::tools::list::server::response::Origin):
/// the template a created tool was made from, or the provider and
/// container id a connected tool joined — and not where a created
/// tool happens to run, which changes. JSON-tagged by `kind`,
/// `created` or `connected`, as the origin is. A tool's count is
/// among the tools ever made with the same one of these.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ToolOrigin {
    /// Made by a create, from a template.
    Created {
        /// The template, by id.
        template: String,
    },
    /// Joined by a connect, to somebody else's container.
    Connected {
        /// The provider the container runs on, as the connect named
        /// it.
        provider: Identity,
        /// The container's id, as the connect named it.
        id: String,
    },
}
