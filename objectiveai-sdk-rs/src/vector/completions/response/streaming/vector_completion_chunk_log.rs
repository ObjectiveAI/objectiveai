//! On-disk shape of a `VectorCompletionChunk` log file.
//!
//! Mirrors [`super::VectorCompletionChunk`] field-for-field. The
//! one type swap is `completions: Vec<AgentCompletionChunk>` →
//! `Vec<IndexedLogReference>` (each per-agent completion
//! is extracted to its own file under `agents/completions/`, with
//! `index` preserved at the reference level).

use schemars::JsonSchema;
use serde::Serialize;

use crate::agent;
use crate::IndexedLogReference;
use crate::vector::completions::response;

#[derive(Debug, Clone, Serialize, JsonSchema)]
#[schemars(
    rename = "vector.completions.response.streaming.VectorCompletionChunkLog"
)]
pub struct VectorCompletionChunkLog {
    pub id: String,
    pub completions: Vec<IndexedLogReference>,
    pub votes: Vec<response::Vote>,
    #[schemars(with = "Vec<f64>")]
    pub scores: Vec<rust_decimal::Decimal>,
    #[schemars(with = "Vec<f64>")]
    pub weights: Vec<rust_decimal::Decimal>,
    pub created: u64,
    pub swarm: String,
    pub object: response::streaming::Object,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub usage: Option<agent::completions::response::Usage>,
}
