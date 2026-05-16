mod executions;
mod inventions;
mod profiles;

pub use executions::*;
pub use inventions::*;
pub use profiles::*;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Result of `functions get`.
///
/// Wire: `{"type":"notification","function":{...GetFunctionResponse...}}`.
#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema)]
#[schemars(rename = "cli.output.notification.functions.Function")]
pub struct Function {
    pub function: crate::functions::response::GetFunctionResponse,
}
