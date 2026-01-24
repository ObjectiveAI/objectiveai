//! Ensemble retrieval client trait definition.

use crate::ctx;

/// Trait for listing ensembles and retrieving usage statistics.
#[async_trait::async_trait]
pub trait Client<CTXEXT> {
    /// Lists all ensembles.
    async fn list(
        &self,
        ctx: ctx::Context<CTXEXT>,
    ) -> Result<
        objectiveai::ensemble::response::ListEnsemble,
        objectiveai::error::ResponseError,
    >;

    /// Retrieves usage statistics for an ensemble by ID.
    async fn get_usage(
        &self,
        ctx: ctx::Context<CTXEXT>,
        id: &str,
    ) -> Result<
        objectiveai::ensemble::response::UsageEnsemble,
        objectiveai::error::ResponseError,
    >;
}
