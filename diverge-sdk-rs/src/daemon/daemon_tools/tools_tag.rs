//! The daemon's tool that tags tools, and how far it reaches.

use serde::{Deserialize, Serialize};

use crate::daemon::endpoints::tools;
use super::Tags;

/// How far the daemon's tool that tags tools reaches: which tools, by a
/// filter, the same shape the list of tools narrows by, read here as a
/// test — see [`daemon_tools`](super) — and which tags, any or only
/// those named.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ToolsTag {
    /// The tools it may put tags on: those the filter passes, judged as
    /// they are before the change. See
    /// [`Filter`](crate::daemon::endpoints::tools::list::client::request::Filter).
    pub tools: tools::list::client::request::Filter,
    /// The tags it may put on: see [`Tags`].
    pub tags: Tags,
}
