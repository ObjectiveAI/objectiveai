//! Usage handler trait definition.

use crate::ctx;
use std::sync::Arc;

/// Trait for handling usage tracking after chat completions.
#[async_trait::async_trait]
pub trait UsageHandler<CTXEXT> {
    /// Called after a chat completion finishes to record usage.
    ///
    /// The `request` is `None` when called from vector completions.
    async fn handle_usage(
        &self,
        ctx: ctx::Context<CTXEXT>,
        request: Option<Arc<objectiveai::chat::completions::request::ChatCompletionCreateParams>>,
        response: objectiveai::chat::completions::response::unary::ChatCompletion,
    );
}
