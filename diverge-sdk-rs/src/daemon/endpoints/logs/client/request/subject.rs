//! Whose log a request reads.

use serde::{Deserialize, Serialize};

use crate::daemon::reference;

/// Whose log: one member, naming the kind, over the reference its
/// family names it by. Later kinds — an account's commands, an
/// administrator's — join here.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[derive(schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Subject {
    /// An agent's log: see [`agent_log`](crate::daemon::agent_log).
    Agent(reference::Agent),
    /// A standalone tool's log: see [`tool_log`](crate::daemon::tool_log).
    StandaloneTool(reference::Tool),
    /// A dependency tool's log: see [`tool_log`](crate::daemon::tool_log).
    DependencyTool(reference::Dependency),
}
