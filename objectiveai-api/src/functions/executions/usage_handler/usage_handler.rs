//! Trait for handling Function execution usage.

use crate::ctx;
use std::sync::Arc;

/// Handler for recording usage after Function execution.
#[async_trait::async_trait]
pub trait UsageHandler<CTXEXT> {
    /// Records usage from a completed Function execution.
    async fn handle_usage(
        &self,
        ctx: ctx::Context<CTXEXT>,
        request: Arc<objectiveai_sdk::functions::executions::request::FunctionExecutionCreateParams>,
        response: objectiveai_sdk::functions::executions::response::unary::FunctionExecution,
    );
}
