//! Trait for handling laboratory execution usage.

use crate::ctx;
use std::sync::Arc;

/// Handler for recording usage after laboratory execution.
#[async_trait::async_trait]
pub trait UsageHandler<CTXEXT> {
    /// Records usage from a completed laboratory execution.
    async fn handle_usage<PC: crate::ctx::persistent_cache::PersistentCacheClient>(
        &self,
        ctx: ctx::Context<CTXEXT, PC>,
        request: Arc<objectiveai_sdk::laboratories::executions::request::LaboratoryExecutionCreateParams>,
        response: objectiveai_sdk::laboratories::executions::response::unary::LaboratoryExecution,
    );
}
