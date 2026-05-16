use crate::vector;
use serde::{Deserialize, Serialize};
use schemars::JsonSchema;

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "vector.completions.cache.CompletionVotes")]
pub struct CompletionVotes {
    pub data: Option<Vec<vector::completions::response::Vote>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "vector.completions.cache.CacheVote")]
pub struct CacheVote {
    pub vote: Option<vector::completions::response::Vote>,
}
