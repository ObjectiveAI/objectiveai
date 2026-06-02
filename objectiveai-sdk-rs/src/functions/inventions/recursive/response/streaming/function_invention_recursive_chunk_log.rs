//! On-disk shape of a `FunctionInventionRecursiveChunk` log file.
//!
//! Mirrors [`super::FunctionInventionRecursiveChunk`] field-for-field,
//! with `inventions: Vec<FunctionInventionChunk>` →
//! `Vec<IndexedLogReference>` (each wrapped invention
//! carries its `index` at the reference level).

use schemars::JsonSchema;
use serde::Serialize;

use crate::agent;
use crate::logs::IndexedLogReference;

#[derive(Debug, Clone, Serialize, JsonSchema)]
#[schemars(
    rename = "functions.inventions.recursive.response.streaming.FunctionInventionRecursiveChunkLog"
)]
pub struct FunctionInventionRecursiveChunkLog {
    pub id: String,
    pub inventions: Vec<IndexedLogReference>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub inventions_errors: Option<bool>,
    pub created: u64,
    pub object: super::Object,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub usage: Option<agent::completions::response::Usage>,
}
