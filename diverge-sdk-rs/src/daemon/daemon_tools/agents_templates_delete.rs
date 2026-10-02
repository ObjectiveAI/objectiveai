//! The daemon's tool that deletes agent templates.

use serde::{Deserialize, Serialize};

use crate::daemon::endpoints::agents;

/// That an agent, or a tool, has the daemon's tool that deletes agent
/// templates, and how far it reaches: each member is a filter, the same
/// shape the list of that family narrows by, read here as a test — see
/// [`daemon_tools`](super). A filter with no member given reaches every
/// one.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct AgentsTemplatesDelete {
    /// The agent templates it may delete: those the filter passes. See
    /// [`Filter`](crate::daemon::endpoints::agents::templates::list::client::request::Filter).
    pub templates: agents::templates::list::client::request::Filter,
}
