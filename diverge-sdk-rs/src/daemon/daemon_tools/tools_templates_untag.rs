//! The daemon's tool that untags tool templates, and how far it reaches.

use serde::{Deserialize, Serialize};

use crate::daemon::endpoints::tools;
use super::Tags;

/// How far the daemon's tool that untags tool templates reaches: which
/// tool templates, by a filter, the same shape the list of tool
/// templates narrows by, read here as a test — see
/// [`daemon_tools`](super) — and which tags, any or only those named.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ToolsTemplatesUntag {
    /// The tool templates it may take tags off: those the filter
    /// passes, judged as they are before the change. See
    /// [`Filter`](crate::daemon::endpoints::tools::templates::list::client::request::Filter).
    pub templates: tools::templates::list::client::request::Filter,
    /// The tags it may take off: see [`Tags`].
    pub tags: Tags,
}
