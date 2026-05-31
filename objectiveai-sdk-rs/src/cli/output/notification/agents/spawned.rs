use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Result of `agents spawn`. Emitted once, immediately before the
/// CLI exits and leaves `objectiveai-cli-stream` running detached
/// to consume the actual completion stream.
///
/// `agent_id` is the local lineage segment of the spawned agent's
/// composite id — paste it directly into `agents read pending` or
/// match against the output of `agents list-active`.
///
/// Wire: `{"type":"notification","agent_id":"<local-id>"}`.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema)]
#[schemars(rename = "cli.output.notification.agents.Spawned")]
pub struct Spawned {
    pub agent_id: String,
}
