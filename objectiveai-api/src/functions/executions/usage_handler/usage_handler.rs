use crate::ctx;
use std::sync::Arc;

#[async_trait::async_trait]
pub trait UsageHandler<CTXEXT> {
    async fn handle_usage(
        &self,
        ctx: ctx::Context<CTXEXT>,
        request: Arc<objectiveai::functions::executions::request::Request>,
        response: objectiveai::functions::executions::response::unary::FunctionExecution,
    );
}
