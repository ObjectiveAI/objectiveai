//! Ensemble client for listing, retrieving, and fetching ensembles.

use crate::ctx;
use std::sync::Arc;

/// Client for ensemble operations.
///
/// Combines a caching fetcher for ensemble definitions with a retrieval
/// client for listing and usage statistics.
pub struct Client<CTXEXT, FENS, RTRVL> {
    /// Caching fetcher for ensemble definitions.
    pub ensemble_fetcher: Arc<super::fetcher::CachingFetcher<CTXEXT, FENS>>,
    /// Client for listing ensembles and getting usage.
    pub retrieval_client: Arc<RTRVL>,
    /// Phantom data for the context extension type.
    pub _ctx_ext: std::marker::PhantomData<CTXEXT>,
}

impl<CTXEXT, FENS, RTRVL> Client<CTXEXT, FENS, RTRVL> {
    /// Creates a new ensemble client.
    pub fn new(
        ensemble_fetcher: Arc<
            super::fetcher::CachingFetcher<CTXEXT, FENS>,
        >,
        retrieval_client: Arc<RTRVL>,
    ) -> Self {
        Self {
            ensemble_fetcher,
            retrieval_client,
            _ctx_ext: std::marker::PhantomData,
        }
    }
}

impl<CTXEXT, FENS, RTRVL> Client<CTXEXT, FENS, RTRVL>
where
    CTXEXT: Send + Sync + 'static,
    FENS: super::fetcher::Fetcher<CTXEXT>
        + Send
        + Sync
        + 'static,
    RTRVL: super::retrieval_client::Client<CTXEXT> + Send + Sync + 'static,
{
    /// Lists all ensembles.
    pub async fn list(
        &self,
        ctx: ctx::Context<CTXEXT>,
    ) -> Result<
        objectiveai::ensemble::response::ListEnsemble,
        objectiveai::error::ResponseError,
    > {
        self.retrieval_client.list(ctx).await
    }

    /// Retrieves an ensemble by its ID.
    ///
    /// Returns a 404 error if the ensemble is not found.
    pub async fn get(
        &self,
        ctx: ctx::Context<CTXEXT>,
        id: &str,
    ) -> Result<
        objectiveai::ensemble::response::GetEnsemble,
        objectiveai::error::ResponseError,
    > {
        self.ensemble_fetcher
            .fetch(ctx, id)
            .await?
            .ok_or_else(|| objectiveai::error::ResponseError {
                code: 404,
                message: serde_json::json!({
                    "kind": "ensemble",
                    "error": "Ensemble not found"
                }),
            })
            .map(|(inner, created)| {
                objectiveai::ensemble::response::GetEnsemble { created, inner }
            })
    }

    /// Retrieves usage statistics for an ensemble.
    pub async fn get_usage(
        &self,
        ctx: ctx::Context<CTXEXT>,
        id: &str,
    ) -> Result<
        objectiveai::ensemble::response::UsageEnsemble,
        objectiveai::error::ResponseError,
    > {
        self.retrieval_client.get_usage(ctx, id).await
    }
}
