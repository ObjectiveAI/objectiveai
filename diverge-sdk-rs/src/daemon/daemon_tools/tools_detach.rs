//! The daemon's tool that detaches tools from agents.

use serde::{Deserialize, Serialize};

use crate::daemon::endpoints::{agents, tools};

/// That an agent, or a tool, has the daemon's tool that detaches tools
/// from agents, and how far it reaches: each member is a filter, the
/// same shape the list of that family narrows by, read here as a test —
/// see [`daemon_tools`](super). A filter with no member given reaches
/// every one.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct ToolsDetach {
    /// The tools it may detach: those the filter passes. See
    /// [`Filter`](crate::daemon::endpoints::tools::list::client::request::Filter).
    pub tools: tools::list::client::request::Filter,
    /// The agents it may detach them from: those the filter passes. See
    /// [`Filter`](crate::daemon::endpoints::agents::list::client::request::Filter).
    pub agents: agents::list::client::request::Filter,
}
