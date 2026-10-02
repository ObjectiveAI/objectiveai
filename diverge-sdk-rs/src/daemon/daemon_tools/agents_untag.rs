//! The daemon's tool that untags agents, and how far it reaches.

use serde::{Deserialize, Serialize};

use crate::daemon::endpoints::agents;
use super::Within;

/// How far the daemon's tool that untags agents reaches, one side each:
/// which agents — any, or only those a filter passes, the same shape
/// the list of agents narrows by, read here as a test, see
/// [`daemon_tools`](super) — and which tags, any or only those named.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AgentsUntag {
    /// The agents it may take tags off: any, or only those the filter
    /// passes, judged as they are before the change. See [`Within`] and
    /// [`Filter`](crate::daemon::endpoints::agents::list::client::request::Filter).
    pub agents: Within<agents::list::client::request::Filter>,
    /// The tags it may take off: any, or only those named. See
    /// [`Within`].
    pub tags: Within<Vec<String>>,
}
