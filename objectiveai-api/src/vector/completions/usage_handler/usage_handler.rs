//! Trait for handling vector completion usage.

use crate::ctx;
use std::sync::Arc;

/// Handles usage tracking after a vector completion completes.
#[async_trait::async_trait]
pub trait UsageHandler<CTXEXT> {
    /// Called after a vector completion finishes to track usage.
    async fn handle_usage(
        &self,
        ctx: ctx::Context<CTXEXT>,
        request: Arc<objectiveai::vector::completions::request::VectorCompletionCreateParams>,
        response: objectiveai::vector::completions::response::unary::VectorCompletion,
    );
}
