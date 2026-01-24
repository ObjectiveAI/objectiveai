use crate::vector;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletionVotes {
    pub data: Option<Vec<vector::completions::response::Vote>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheVote {
    pub vote: Option<vector::completions::response::Vote>,
}
