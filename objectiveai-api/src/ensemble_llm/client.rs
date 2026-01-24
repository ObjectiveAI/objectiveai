//! Ensemble LLM client for listing, retrieving, and fetching Ensemble LLMs.

use crate::ctx;
use std::sync::Arc;

/// Client for Ensemble LLM operations.
///
/// Combines a caching fetcher for Ensemble LLM definitions with a retrieval
/// client for listing and usage statistics.
pub struct Client<CTXEXT, FENSLLM, RTRVL> {
    /// Caching fetcher for Ensemble LLM definitions.
    pub ensemble_llm_fetcher: Arc<super::fetcher::CachingFetcher<CTXEXT, FENSLLM>>,
    /// Client for listing Ensemble LLMs and getting usage.
    pub retrieval_client: Arc<RTRVL>,
    /// Phantom data for the context extension type.
    pub _ctx_ext: std::marker::PhantomData<CTXEXT>,
}

impl<CTXEXT, FENSLLM, RTRVL> Client<CTXEXT, FENSLLM, RTRVL> {
    /// Creates a new Ensemble LLM client.
    pub fn new(
        ensemble_llm_fetcher: Arc<
            super::fetcher::CachingFetcher<
                CTXEXT,
                FENSLLM,
            >,
        >,
        retrieval_client: Arc<RTRVL>,
    ) -> Self {
        Self {
            ensemble_llm_fetcher,
            retrieval_client,
            _ctx_ext: std::marker::PhantomData,
        }
    }
}

impl<CTXEXT, FENSLLM, RTRVL> Client<CTXEXT, FENSLLM, RTRVL>
where
    CTXEXT: Send + Sync + 'static,
    FENSLLM: super::fetcher::Fetcher<CTXEXT>
        + Send
        + Sync
        + 'static,
    RTRVL: super::retrieval_client::Client<CTXEXT> + Send + Sync + 'static,
{
    /// Lists all Ensemble LLMs.
    pub async fn list(
        &self,
        ctx: ctx::Context<CTXEXT>,
    ) -> Result<
        objectiveai::ensemble_llm::response::ListEnsembleLlm,
        objectiveai::error::ResponseError,
    > {
        self.retrieval_client.list(ctx).await
    }

    /// Retrieves an Ensemble LLM by its ID.
    ///
    /// Returns a 404 error if the Ensemble LLM is not found.
    pub async fn get(
        &self,
        ctx: ctx::Context<CTXEXT>,
        id: &str,
    ) -> Result<
        objectiveai::ensemble_llm::response::GetEnsembleLlm,
        objectiveai::error::ResponseError,
    > {
        self.ensemble_llm_fetcher
            .fetch(ctx, id)
            .await?
            .ok_or_else(|| objectiveai::error::ResponseError {
                code: 404,
                message: serde_json::json!({
                    "kind": "ensemble_llm",
                    "error": "Ensemble LLM not found"
                }),
            })
            .map(|(inner, created)| {
                objectiveai::ensemble_llm::response::GetEnsembleLlm {
                    created,
                    inner,
                }
            })
    }

    /// Retrieves usage statistics for an Ensemble LLM.
    pub async fn get_usage(
        &self,
        ctx: ctx::Context<CTXEXT>,
        id: &str,
    ) -> Result<
        objectiveai::ensemble_llm::response::UsageEnsembleLlm,
        objectiveai::error::ResponseError,
    > {
        self.retrieval_client.get_usage(ctx, id).await
    }
}
