//! The daemon's tool that deletes tools.

use serde::{Deserialize, Serialize};

use crate::daemon::endpoints::tools;

/// That an agent, or a tool, has the daemon's tool that deletes tools,
/// and how far it reaches: each member is a filter, the same shape the
/// list of that family narrows by, read here as a test — see
/// [`daemon_tools`](super). A filter with no member given reaches every
/// one.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct ToolsDelete {
    /// The tools it may delete: those the filter passes. See
    /// [`Filter`](crate::daemon::endpoints::tools::list::client::request::Filter).
    pub tools: tools::list::client::request::Filter,
}
