//! `LogRow` — the writer's typed shadow-map value. One variant per
//! `(table, body shape)` pair. The writer keys its dedup map by the
//! chunk's `id` field and stores the last-written `LogRow` it
//! produced for that id; an incoming chunk only writes the row if the
//! freshly-stripped `LogRow` differs from the cached one.

use objectiveai_sdk::agent::completions::request::AgentCompletionCreateParamsLog;
use objectiveai_sdk::agent::completions::response::streaming::AgentCompletionChunkLog;
use objectiveai_sdk::functions::executions::request::FunctionExecutionCreateParamsLog;
use objectiveai_sdk::functions::executions::response::streaming::FunctionExecutionChunkLog;
use objectiveai_sdk::logs::LogTable;
use objectiveai_sdk::vector::completions::request::VectorCompletionCreateParams;
use objectiveai_sdk::vector::completions::response::streaming::VectorCompletionChunkLog;

#[derive(Debug, Clone)]
pub enum LogRow {
    AgentCompletionRequest(AgentCompletionCreateParamsLog),
    AgentCompletionResponse(AgentCompletionChunkLog),
    /// Vector-completion request bodies don't currently strip — the
    /// shape ships as the bare `VectorCompletionCreateParams` since it
    /// has no embedded sub-completions. Lives in `logs.vector_completion_requests`.
    VectorCompletionRequest(VectorCompletionCreateParams),
    VectorCompletionResponse(VectorCompletionChunkLog),
    FunctionExecutionRequest(FunctionExecutionCreateParamsLog),
    FunctionExecutionResponse(FunctionExecutionChunkLog),
}

impl LogRow {
    pub fn table(&self) -> LogTable {
        match self {
            LogRow::AgentCompletionRequest(_) => LogTable::AgentCompletionRequest,
            LogRow::AgentCompletionResponse(_) => LogTable::AgentCompletionResponse,
            LogRow::VectorCompletionRequest(_) => LogTable::VectorCompletionRequest,
            LogRow::VectorCompletionResponse(_) => LogTable::VectorCompletionResponse,
            LogRow::FunctionExecutionRequest(_) => LogTable::FunctionExecutionRequest,
            LogRow::FunctionExecutionResponse(_) => LogTable::FunctionExecutionResponse,
        }
    }
}
