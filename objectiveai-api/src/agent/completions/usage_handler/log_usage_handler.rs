//! Simple logging usage handler for development.

use crate::ctx;
use std::sync::Arc;

/// Usage handler that logs completion costs to stdout.
pub struct LogUsageHandler;

impl<CTXEXT> super::UsageHandler<CTXEXT> for LogUsageHandler
where
    CTXEXT: Send + Sync + 'static,
{
    fn handle_usage(
        &self,
        ctx: ctx::Context<CTXEXT>,
        _request: Arc<objectiveai_sdk::agent::completions::request::AgentCompletionCreateParams>,
        response: objectiveai_sdk::agent::completions::response::unary::AgentCompletion,
    ) -> impl std::future::Future<Output = ()> + Send + 'static {
        async move {
            if !ctx.suppress_output {
                println!(
                    "[{}] cost: {}",
                    response.id.as_str(),
                    response.usage.total_cost,
                );
            }
        }
    }
}
