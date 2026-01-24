//! Functions client implementation.

use crate::ctx;
use std::sync::Arc;

/// Client for function operations.
pub struct Client<CTXEXT, FFN, RTRVL> {
    /// Fetcher for Function definitions.
    pub function_fetcher: Arc<FFN>,
    /// Client for listing functions and getting usage statistics.
    pub retrieval_client: Arc<RTRVL>,
    pub _ctx_ext: std::marker::PhantomData<CTXEXT>,
}

impl<CTXEXT, FFN, RTRVL> Client<CTXEXT, FFN, RTRVL> {
    /// Creates a new functions client.
    pub fn new(
        function_fetcher: Arc<FFN>,
        retrieval_client: Arc<RTRVL>,
    ) -> Self {
        Self {
            function_fetcher,
            retrieval_client,
            _ctx_ext: std::marker::PhantomData,
        }
    }
}

impl<CTXEXT, FFN, RTRVL> Client<CTXEXT, FFN, RTRVL>
where
    CTXEXT: Send + Sync + 'static,
    FFN: super::function_fetcher::Fetcher<CTXEXT> + Send + Sync + 'static,
    RTRVL: super::retrieval_client::Client<CTXEXT> + Send + Sync + 'static,
{
    /// Lists functions.
    pub async fn list_functions(
        &self,
        ctx: ctx::Context<CTXEXT>,
    ) -> Result<
        objectiveai::functions::response::ListFunction,
        objectiveai::error::ResponseError,
    > {
        self.retrieval_client.list_functions(ctx).await
    }

    /// Retrieves a function by owner/repository/commit.
    pub async fn get_function(
        &self,
        ctx: ctx::Context<CTXEXT>,
        owner: &str,
        repository: &str,
        commit: Option<&str>,
    ) -> Result<
        objectiveai::functions::response::GetFunction,
        objectiveai::error::ResponseError,
    > {
        self.function_fetcher
            .fetch(ctx, owner, repository, commit)
            .await?
            .ok_or_else(|| objectiveai::error::ResponseError {
                code: 404,
                message: serde_json::json!({
                    "kind": "functions",
                    "error": "Function not found"
                }),
            })
    }

    /// Retrieves usage statistics for a function.
    pub async fn get_function_usage(
        &self,
        ctx: ctx::Context<CTXEXT>,
        owner: &str,
        repository: &str,
        commit: Option<&str>,
    ) -> Result<
        objectiveai::functions::response::UsageFunction,
        objectiveai::error::ResponseError,
    > {
        self.retrieval_client
            .get_function_usage(ctx, owner, repository, commit)
            .await
    }
}
