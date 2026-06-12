//! Wire shapes for events the viewer client POSTs to a remote viewer's
//! HTTP server. The api server constructs these and pushes them through
//! [`super::Client`]; standalone SDK consumers can do the same.

use std::sync::Arc;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "http.viewer.ResponseError")]
pub struct ResponseError {
    pub id: String,
    #[serde(flatten)]
    pub inner: crate::error::ResponseError,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "http.viewer.AgentCompletionCreateParams")]
pub struct AgentCompletionCreateParams {
    pub id: String,
    #[serde(flatten)]
    pub inner:
        Arc<crate::agent::completions::request::AgentCompletionCreateParams>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "http.viewer.AgentCompletionRequest")]
#[serde(untagged)]
pub enum AgentCompletionRequest {
    #[schemars(title = "Begin")]
    Begin(AgentCompletionCreateParams),
    #[schemars(title = "Continue")]
    Continue(
        crate::agent::completions::response::streaming::AgentCompletionChunk,
    ),
    #[schemars(title = "Error")]
    Error(ResponseError),
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "http.viewer.FunctionExecutionCreateParams")]
pub struct FunctionExecutionCreateParams {
    pub id: String,
    #[serde(flatten)]
    pub inner: Arc<
        crate::functions::executions::request::FunctionExecutionCreateParams,
    >,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "http.viewer.FunctionExecutionRequest")]
#[serde(untagged)]
pub enum FunctionExecutionRequest {
    #[schemars(title = "Begin")]
    Begin(FunctionExecutionCreateParams),
    #[schemars(title = "Continue")]
    Continue(crate::functions::executions::response::streaming::FunctionExecutionChunk),
    #[schemars(title = "Error")]
    Error(ResponseError),
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "http.viewer.Request")]
#[serde(untagged)]
pub enum Request {
    #[schemars(title = "AgentCompletion")]
    AgentCompletion(AgentCompletionRequest),
    #[schemars(title = "FunctionExecution")]
    FunctionExecution(FunctionExecutionRequest),
}
