//! Whose log a request reads.

use serde::{Deserialize, Serialize};

use crate::daemon::endpoints::accounts;
use crate::daemon::reference;
use crate::shared::Identity;

/// Whose log: `type` naming the kind, and beside it the members of the
/// reference its family names it by — `{"type":"agent","name":"alice"}`,
/// `{"type":"account","issuer":…,"subject":…}`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[derive(schemars::JsonSchema)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Subject {
    /// An agent's log: see [`agent_log`](crate::daemon::agent_log).
    Agent(reference::Agent),
    /// A standalone tool's log: see [`tool_log`](crate::daemon::tool_log).
    StandaloneTool(reference::Tool),
    /// A dependency tool's log: see [`tool_log`](crate::daemon::tool_log).
    DependencyTool(reference::Dependency),
    /// An account's command log: see
    /// [`command_log`](crate::daemon::command_log). Read under the
    /// accounts family's `logs` grant.
    Account(accounts::Reference),
    /// An administrator's command log, by the identity the daemon's
    /// configuration names: see
    /// [`command_log`](crate::daemon::command_log). Read by
    /// administrators alone.
    Admin(Identity),
}
