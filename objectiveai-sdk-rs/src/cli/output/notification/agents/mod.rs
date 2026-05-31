mod active;
mod agent_items;
mod messaged;
mod spawned;

pub use active::*;
pub use agent_items::*;
pub use messaged::*;
pub use spawned::*;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Result of `agents get`.
///
/// Wire: `{"type":"notification","agent":{...GetAgentResponse...}}`.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema)]
#[schemars(rename = "cli.output.notification.agents.Agent")]
pub struct Agent {
    pub agent: crate::agent::response::GetAgentResponse,
}
