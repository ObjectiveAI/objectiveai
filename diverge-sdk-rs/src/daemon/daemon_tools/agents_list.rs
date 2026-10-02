//! The daemon's tool that lists agents.

use serde::{Deserialize, Serialize};

use crate::daemon::endpoints::agents;

/// That an agent, or a tool, has the daemon's tool that lists agents,
/// and how far it reaches: each member is a filter, the same shape the
/// list of that family narrows by, read here as a test — see
/// [`daemon_tools`](super). A filter with no member given reaches every
/// one.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct AgentsList {
    /// The agents it reaches: those the filter passes. Its own list
    /// requests narrow within these. See
    /// [`Filter`](crate::daemon::endpoints::agents::list::client::request::Filter).
    pub agents: agents::list::client::request::Filter,
}
