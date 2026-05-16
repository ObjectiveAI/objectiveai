use crate::ctx;
use std::sync::Arc;

/// Handler for recording usage after recursive Function invention.
#[async_trait::async_trait]
pub trait UsageHandler<CTXEXT> {
    /// Records usage from a completed recursive Function invention.
    async fn handle_usage<PC: crate::ctx::persistent_cache::PersistentCacheClient>(
        &self,
        ctx: ctx::Context<CTXEXT, PC>,
        request: Arc<objectiveai_sdk::functions::inventions::recursive::request::FunctionInventionRecursiveCreateParams>,
        response: objectiveai_sdk::functions::inventions::recursive::response::unary::FunctionInventionRecursive,
    );
}
