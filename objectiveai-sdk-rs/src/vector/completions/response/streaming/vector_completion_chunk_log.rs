//! `VectorCompletionChunkLog` — postgres-log shape of
//! [`super::VectorCompletionChunk`].
//!
//! Mirrors the wire chunk with one swap: `completions:
//! Vec<AgentCompletionChunk>` → `Vec<`[`super::AgentCompletionLogRef`]`>`
//! — each per-agent slot becomes a typed ref into
//! `logs.agent_completion_responses`, carrying the wire-side `index`
//! and any wrapper-level error.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::agent;
use crate::vector::completions::response;

use super::AgentCompletionLogRef;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[schemars(
    rename = "vector.completions.response.streaming.VectorCompletionChunkLog"
)]
pub struct VectorCompletionChunkLog {
    pub id: String,
    pub completions: Vec<AgentCompletionLogRef>,
    pub votes: Vec<response::Vote>,
    #[serde(deserialize_with = "crate::serde_util::vec_decimal")]
    #[schemars(with = "Vec<f64>")]
    pub scores: Vec<rust_decimal::Decimal>,
    #[serde(deserialize_with = "crate::serde_util::vec_decimal")]
    #[schemars(with = "Vec<f64>")]
    pub weights: Vec<rust_decimal::Decimal>,
    pub created: u64,
    pub swarm: String,
    pub object: response::streaming::Object,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub usage: Option<agent::completions::response::Usage>,
}
