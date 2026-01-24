//! Ensemble LLM fetcher trait definition.

use crate::ctx;

/// Trait for fetching Ensemble LLM definitions by ID.
#[async_trait::async_trait]
pub trait Fetcher<CTXEXT> {
    /// Fetches an Ensemble LLM by its ID.
    ///
    /// Returns `Ok(None)` if the Ensemble LLM is not found.
    /// Returns the Ensemble LLM and its creation timestamp if found.
    async fn fetch(
        &self,
        ctx: ctx::Context<CTXEXT>,
        id: &str,
    ) -> Result<
        Option<(objectiveai::ensemble_llm::EnsembleLlm, u64)>,
        objectiveai::error::ResponseError,
    >;
}
