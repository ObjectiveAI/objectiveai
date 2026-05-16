//! Trait for fetching votes from the global cache.

use crate::ctx;

/// Fetches votes from the global ObjectiveAI vote cache.
#[async_trait::async_trait]
pub trait Fetcher<CTXEXT> {
    /// Requests a cached vote matching the given parameters.
    ///
    /// Returns None if no cached vote exists.
    async fn fetch<PC: crate::ctx::persistent_cache::PersistentCacheClient>(
        &self,
        ctx: ctx::Context<CTXEXT, PC>,
        agent: &objectiveai_sdk::agent::InlineAgentBaseWithFallbacksOrRemote,
        messages: &[objectiveai_sdk::agent::completions::message::Message],
        responses: &[objectiveai_sdk::agent::completions::message::RichContent],
    ) -> Result<
        Option<objectiveai_sdk::vector::completions::response::Vote>,
        objectiveai_sdk::error::ResponseError,
    >;
}
