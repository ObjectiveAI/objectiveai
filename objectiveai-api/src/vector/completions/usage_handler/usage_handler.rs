//! Trait for handling vector completion usage.

use crate::ctx;
use std::sync::Arc;

/// Handles usage tracking after a vector completion completes.
#[async_trait::async_trait]
pub trait UsageHandler<CTXEXT> {
    /// Called after a vector completion finishes to track usage.
    async fn handle_usage<PC: crate::ctx::persistent_cache::PersistentCacheClient>(
        &self,
        ctx: ctx::Context<CTXEXT, PC>,
        request: Arc<objectiveai_sdk::vector::completions::request::VectorCompletionCreateParams>,
        response: objectiveai_sdk::vector::completions::response::unary::VectorCompletion,
    );
}
