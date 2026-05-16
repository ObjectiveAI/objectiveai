use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Result of `functions inventions state get`.
///
/// Wire: `{"type":"notification","state":{...GetFunctionInventionStateResponse...}}`.
#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema)]
#[schemars(rename = "cli.output.notification.functions.inventions.State")]
pub struct State {
    pub state: crate::functions::inventions::state::response::GetFunctionInventionStateResponse,
}
