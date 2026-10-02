//! The daemon's tool that tags agents, and how far it reaches.

use serde::{Deserialize, Serialize};

use crate::daemon::endpoints::agents;
use super::Tags;

/// How far the daemon's tool that tags agents reaches: which agents, by
/// a filter, the same shape the list of agents narrows by, read here as
/// a test — see [`daemon_tools`](super) — and which tags, any or only
/// those named.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AgentsTag {
    /// The agents it may put tags on: those the filter passes, judged
    /// as they are before the change. See
    /// [`Filter`](crate::daemon::endpoints::agents::list::client::request::Filter).
    pub agents: agents::list::client::request::Filter,
    /// The tags it may put on: see [`Tags`].
    pub tags: Tags,
}
