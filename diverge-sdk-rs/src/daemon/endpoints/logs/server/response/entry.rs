//! One entry of a log, in its subject's kind.

use serde::{Deserialize, Serialize};

use crate::daemon::{agent_log, tool_log};

/// One entry of the log read: one member, naming the subject's kind,
/// over the entry as that kind's book keeps it. Every entry of one
/// read is of the kind its [`Subject`] named.
///
/// [`Subject`]: crate::daemon::endpoints::logs::client::request::Subject
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[derive(schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Entry {
    /// An agent's.
    Agent(agent_log::ItemWrapper),
    /// A standalone tool's.
    StandaloneTool(tool_log::ItemWrapper),
    /// A dependency tool's.
    DependencyTool(tool_log::ItemWrapper),
}
