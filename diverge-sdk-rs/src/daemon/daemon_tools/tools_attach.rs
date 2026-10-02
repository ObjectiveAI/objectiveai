//! The daemon's tool that attaches tools to agents.

use serde::{Deserialize, Serialize};

use crate::daemon::endpoints::{agents, tools};

/// That an agent, or a tool, has the daemon's tool that attaches tools
/// to agents, and how far it reaches: each member is a filter, the same
/// shape the list of that family narrows by, read here as a test — see
/// [`daemon_tools`](super). A filter with no member given reaches every
/// one.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct ToolsAttach {
    /// The tools it may attach: those the filter passes. See
    /// [`Filter`](crate::daemon::endpoints::tools::list::client::request::Filter).
    pub tools: tools::list::client::request::Filter,
    /// The agents it may attach them to: those the filter passes. See
    /// [`Filter`](crate::daemon::endpoints::agents::list::client::request::Filter).
    pub agents: agents::list::client::request::Filter,
}
