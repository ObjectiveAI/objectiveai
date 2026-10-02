//! The daemon's tool that tags tool templates, and how far it reaches.

use serde::{Deserialize, Serialize};

use crate::daemon::endpoints::tools;
use super::Within;

/// How far the daemon's tool that tags tool templates reaches, one side
/// each: which tool templates — any, or only those a filter passes, the
/// same shape the list of tool templates narrows by, read here as a
/// test, see [`daemon_tools`](super) — and which tags, any or only
/// those named.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ToolsTemplatesTag {
    /// The tool templates it may put tags on: any, or only those the
    /// filter passes, judged as they are before the change. See
    /// [`Within`] and
    /// [`Filter`](crate::daemon::endpoints::tools::templates::list::client::request::Filter).
    pub templates: Within<tools::templates::list::client::request::Filter>,
    /// The tags it may put on: any, or only those named. See
    /// [`Within`].
    pub tags: Within<Vec<String>>,
}
