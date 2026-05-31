//! On-disk shape of a `FunctionInventionChunk` log file.
//!
//! Mirrors [`super::FunctionInventionChunk`] field-for-field, with
//! `completions: Vec<AgentCompletionChunk>` →
//! `Vec<indexed_reference::LogReference>` (each per-agent completion
//! in its own file under `agents/completions/`, with `index` at the
//! reference level).

use schemars::JsonSchema;
use serde::Serialize;

use crate::agent;
use crate::error;
use crate::filesystem::logs::indexed_reference;
use crate::functions;

#[derive(Debug, Clone, Serialize, JsonSchema)]
#[schemars(rename = "functions.inventions.response.streaming.FunctionInventionChunkLog")]
pub struct FunctionInventionChunkLog {
    pub id: String,
    pub completions: Vec<indexed_reference::LogReference>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub state: Option<functions::inventions::State>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub path: Option<crate::RemotePath>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub function: Option<functions::FullRemoteFunction>,
    pub created: u64,
    pub object: super::Object,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub usage: Option<agent::completions::response::Usage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub error: Option<error::ResponseError>,
}
