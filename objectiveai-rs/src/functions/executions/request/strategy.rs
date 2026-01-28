use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Strategy {
    /// Scalar or Vector
    Default,
    /// Vector
    SwissSystem {
        /// How many vector responses for each execution
        pool: Option<usize>, // default is 10
        /// How many sequential rounds of comparison
        rounds: Option<usize>, // default is 3
    },
}
