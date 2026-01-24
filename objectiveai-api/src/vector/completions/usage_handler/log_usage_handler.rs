//! Simple logging implementation of usage handler.

use crate::ctx;
use std::sync::Arc;

/// A usage handler that logs completion costs to stdout.
pub struct LogUsageHandler;

#[async_trait::async_trait]
impl<CTXEXT> super::UsageHandler<CTXEXT> for LogUsageHandler
where
    CTXEXT: Send + Sync + 'static,
{
    async fn handle_usage(
        &self,
        _ctx: ctx::Context<CTXEXT>,
        _request: Arc<objectiveai::vector::completions::request::VectorCompletionCreateParams>,
        response: objectiveai::vector::completions::response::unary::VectorCompletion,
    ) {
        println!(
            "[{}] cost: {}",
            response.id.as_str(),
            response.usage.total_cost,
        );
    }
}
