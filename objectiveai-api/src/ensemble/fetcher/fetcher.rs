//! Ensemble fetcher trait definition.

use crate::ctx;

/// Trait for fetching ensemble definitions by ID.
#[async_trait::async_trait]
pub trait Fetcher<CTXEXT> {
    /// Fetches an ensemble by its ID.
    ///
    /// Returns `Ok(None)` if the ensemble is not found.
    /// Returns the ensemble and its creation timestamp if found.
    async fn fetch(
        &self,
        ctx: ctx::Context<CTXEXT>,
        id: &str,
    ) -> Result<
        Option<(objectiveai::ensemble::Ensemble, u64)>,
        objectiveai::error::ResponseError,
    >;
}
