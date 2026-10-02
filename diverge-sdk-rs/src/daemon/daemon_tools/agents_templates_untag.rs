//! The daemon's tool that untags agent templates, and how far it reaches.

use serde::{Deserialize, Serialize};

use crate::daemon::endpoints::agents;
use super::Within;

/// How far the daemon's tool that untags agent templates reaches, one
/// side each: which agent templates — any, or only those a filter
/// passes, the same shape the list of agent templates narrows by, read
/// here as a test, see [`daemon_tools`](super) — and which tags, any or
/// only those named.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AgentsTemplatesUntag {
    /// The agent templates it may take tags off: any, or only those the
    /// filter passes, judged as they are before the change. See
    /// [`Within`] and
    /// [`Filter`](crate::daemon::endpoints::agents::templates::list::client::request::Filter).
    pub templates: Within<agents::templates::list::client::request::Filter>,
    /// The tags it may take off: any, or only those named. See
    /// [`Within`].
    pub tags: Within<Vec<String>>,
}
