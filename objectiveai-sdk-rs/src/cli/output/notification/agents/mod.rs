mod completions;

pub use completions::*;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Result of `agents get`.
///
/// Wire: `{"type":"notification","agent":{...GetAgentResponse...}}`.
#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema)]
#[schemars(rename = "cli.output.notification.agents.Agent")]
pub struct Agent {
    pub agent: crate::agent::response::GetAgentResponse,
}
