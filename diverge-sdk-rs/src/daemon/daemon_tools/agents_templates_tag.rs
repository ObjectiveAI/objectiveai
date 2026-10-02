//! The daemon's tool that tags agent templates, and how far it reaches.

use serde::{Deserialize, Serialize};

use crate::daemon::endpoints::agents;
use super::Tags;

/// How far the daemon's tool that tags agent templates reaches: which
/// agent templates, by a filter, the same shape the list of agent
/// templates narrows by, read here as a test — see
/// [`daemon_tools`](super) — and which tags, any or only those named.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AgentsTemplatesTag {
    /// The agent templates it may put tags on: those the filter passes,
    /// judged as they are before the change. See
    /// [`Filter`](crate::daemon::endpoints::agents::templates::list::client::request::Filter).
    pub templates: agents::templates::list::client::request::Filter,
    /// The tags it may put on: see [`Tags`].
    pub tags: Tags,
}
