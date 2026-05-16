use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Result of `swarms get`.
///
/// Wire: `{"type":"notification","swarm":{...GetSwarmResponse...}}`.
#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema)]
#[schemars(rename = "cli.output.notification.swarms.Swarm")]
pub struct Swarm {
    pub swarm: crate::swarm::response::GetSwarmResponse,
}
