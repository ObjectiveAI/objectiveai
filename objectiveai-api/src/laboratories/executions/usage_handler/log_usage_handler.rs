//! Logging implementation of the usage handler.

use crate::ctx;
use std::sync::Arc;

/// Usage handler that logs laboratory execution costs to stdout.
pub struct LogUsageHandler;

#[async_trait::async_trait]
impl<CTXEXT> super::UsageHandler<CTXEXT> for LogUsageHandler
where
    CTXEXT: Send + Sync + 'static,
{
    async fn handle_usage<PC: crate::ctx::persistent_cache::PersistentCacheClient>(
        &self,
        ctx: ctx::Context<CTXEXT, PC>,
        _request: Arc<objectiveai_sdk::laboratories::executions::request::LaboratoryExecutionCreateParams>,
        response: objectiveai_sdk::laboratories::executions::response::unary::LaboratoryExecution,
    ) {
        if !ctx.suppress_output {
            println!(
                "[{}] cost: {}",
                response.id.as_str(),
                response.usage.total_cost,
            );
        }
    }
}
