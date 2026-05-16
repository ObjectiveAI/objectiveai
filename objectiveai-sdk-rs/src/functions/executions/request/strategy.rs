use serde::{Deserialize, Serialize};
use schemars::JsonSchema;

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "type", rename_all = "snake_case")]
#[schemars(rename = "functions.executions.request.Strategy")]
pub enum Strategy {
    /// Scalar or Vector
    #[schemars(title = "Default")]
    Default,
    /// Vector
    #[schemars(title = "SwissSystem")]
    SwissSystem {
        /// How many vector responses for each execution
        pool: Option<usize>, // default is 10
        /// How many sequential rounds of comparison
        rounds: Option<usize>, // default is 3
    },
}
