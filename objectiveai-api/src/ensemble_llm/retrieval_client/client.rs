//! Ensemble LLM retrieval client trait definition.

use crate::ctx;

/// Trait for listing Ensemble LLMs and retrieving usage statistics.
#[async_trait::async_trait]
pub trait Client<CTXEXT> {
    /// Lists all Ensemble LLMs.
    async fn list(
        &self,
        ctx: ctx::Context<CTXEXT>,
    ) -> Result<
        objectiveai::ensemble_llm::response::ListEnsembleLlm,
        objectiveai::error::ResponseError,
    >;

    /// Retrieves usage statistics for an Ensemble LLM by ID.
    async fn get_usage(
        &self,
        ctx: ctx::Context<CTXEXT>,
        id: &str,
    ) -> Result<
        objectiveai::ensemble_llm::response::UsageEnsembleLlm,
        objectiveai::error::ResponseError,
    >;
}
