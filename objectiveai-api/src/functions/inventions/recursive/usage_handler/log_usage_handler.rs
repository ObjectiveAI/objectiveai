use crate::ctx;
use std::sync::Arc;

/// Usage handler that logs recursive invention costs to stdout.
pub struct LogUsageHandler;

#[async_trait::async_trait]
impl<CTXEXT> super::UsageHandler<CTXEXT> for LogUsageHandler
where
    CTXEXT: Send + Sync + 'static,
{
    async fn handle_usage<PC: crate::ctx::persistent_cache::PersistentCacheClient>(
        &self,
        ctx: ctx::Context<CTXEXT, PC>,
        _request: Arc<objectiveai_sdk::functions::inventions::recursive::request::FunctionInventionRecursiveCreateParams>,
        response: objectiveai_sdk::functions::inventions::recursive::response::unary::FunctionInventionRecursive,
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
