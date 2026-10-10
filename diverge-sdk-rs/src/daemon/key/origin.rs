//! The part of a tool's origin that never changes.

use serde::{Deserialize, Serialize};

use crate::daemon::reference;

/// What a tool was made with, as much of it as is fixed for its life:
/// the template a created tool was made from, or the daemon and the
/// tool a connected tool joined — and not where a created tool
/// happens to run, which changes. JSON-tagged by `kind`, `created` or
/// `connected`, as the tools list's
/// [`Origin`](crate::daemon::endpoints::tools::list::server::response::Origin)
/// is. A tool's index counts among the tools ever made with the same
/// one of these.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Origin {
    /// Made by a create, from a template.
    Created {
        /// The template, by id.
        template: String,
    },
    /// Joined by a connect, to another daemon's tool.
    Connected {
        /// The daemon the tool is on, by the name of its record, as
        /// the connect named it.
        daemon: String,
        /// The tool, as that daemon names it and as the connect named
        /// it: see [`reference::Tool`].
        tool: Box<reference::Tool>,
    },
}
