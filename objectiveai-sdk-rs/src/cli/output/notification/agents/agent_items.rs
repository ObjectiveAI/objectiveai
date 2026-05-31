use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// One spawned agent's queue items, paired with the agent_id they
/// belong to. Emitted once per positional arg by the `agents
/// read *` subcommand family — currently `read pending`, soon
/// `read all`.
///
/// `agent_id` is the sub-id (lineage-relative) the caller passed
/// — exactly the form the caller used as the positional arg.
/// `items` may be empty: an empty list signals "you asked and
/// there was nothing matching" rather than the agent being
/// absent.
///
/// Wire: `{"type":"notification","value":{"agent_id":"<sub>","items":[…]}}`.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema)]
#[schemars(rename = "cli.output.notification.agents.AgentItems")]
pub struct AgentItems {
    pub agent_id: String,
    pub items: Vec<crate::filesystem::logs::queue::QueueItem>,
}
