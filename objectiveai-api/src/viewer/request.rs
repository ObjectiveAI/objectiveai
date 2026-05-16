use std::sync::Arc;
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResponseError {
    pub id: String,
    #[serde(flatten)]
    pub inner: objectiveai_sdk::error::ResponseError,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentCompletionCreateParams {
    pub id: String,
    #[serde(flatten)]
    pub inner: Arc<objectiveai_sdk::agent::completions::request::AgentCompletionCreateParams>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum AgentCompletionRequest {
    Begin(AgentCompletionCreateParams),
    Continue(objectiveai_sdk::agent::completions::response::streaming::AgentCompletionChunk),
    Error(ResponseError),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionExecutionCreateParams {
    pub id: String,
    #[serde(flatten)]
    pub inner: Arc<objectiveai_sdk::functions::executions::request::FunctionExecutionCreateParams>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum FunctionExecutionRequest {
    Begin(FunctionExecutionCreateParams),
    Continue(objectiveai_sdk::functions::executions::response::streaming::FunctionExecutionChunk),
    Error(ResponseError),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionInventionRecursiveCreateParams {
    pub id: String,
    #[serde(flatten)]
    pub inner: Arc<objectiveai_sdk::functions::inventions::recursive::request::FunctionInventionRecursiveCreateParams>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum FunctionInventionRecursiveRequest {
    Begin(FunctionInventionRecursiveCreateParams),
    Continue(objectiveai_sdk::functions::inventions::recursive::response::streaming::FunctionInventionRecursiveChunk),
    Error(ResponseError),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LaboratoryExecutionCreateParams {
    pub id: String,
    #[serde(flatten)]
    pub inner: Arc<objectiveai_sdk::laboratories::executions::request::LaboratoryExecutionCreateParams>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum LaboratoryExecutionRequest {
    Begin(LaboratoryExecutionCreateParams),
    Continue(objectiveai_sdk::laboratories::executions::response::streaming::LaboratoryExecutionChunk),
    Error(ResponseError),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Request {
    AgentCompletion(AgentCompletionRequest),
    FunctionExecution(FunctionExecutionRequest),
    FunctionInventionRecursive(FunctionInventionRecursiveRequest),
    LaboratoryExecution(LaboratoryExecutionRequest),
}
