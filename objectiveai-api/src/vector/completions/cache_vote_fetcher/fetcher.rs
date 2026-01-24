//! Trait for fetching votes from the global cache.

use crate::ctx;

/// Fetches votes from the global ObjectiveAI vote cache.
#[async_trait::async_trait]
pub trait Fetcher<CTXEXT> {
    /// Requests a cached vote matching the given parameters.
    ///
    /// Returns None if no cached vote exists.
    async fn fetch(
        &self,
        ctx: ctx::Context<CTXEXT>,
        model: &objectiveai::chat::completions::request::Model,
        models: Option<&[objectiveai::chat::completions::request::Model]>,
        messages: &[objectiveai::chat::completions::request::Message],
        tools: Option<&[objectiveai::chat::completions::request::Tool]>,
        responses: &[objectiveai::chat::completions::request::RichContent],
    ) -> Result<
        Option<objectiveai::vector::completions::response::Vote>,
        objectiveai::error::ResponseError,
    >;
}
