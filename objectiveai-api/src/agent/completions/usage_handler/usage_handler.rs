//! Usage handler trait definition.

use crate::ctx;
use std::sync::Arc;

/// Trait for handling usage tracking after chat completions.
pub trait UsageHandler<CTXEXT> {
    /// Called after a chat completion finishes to record usage.
    ///
    /// The `request` is `None` when called from vector completions.
    fn handle_usage(
        &self,
        ctx: ctx::Context<CTXEXT>,
        request: Arc<objectiveai_sdk::agent::completions::request::AgentCompletionCreateParams>,
        response: objectiveai_sdk::agent::completions::response::unary::AgentCompletion,
    ) -> impl std::future::Future<Output = ()> + Send + 'static;
}
