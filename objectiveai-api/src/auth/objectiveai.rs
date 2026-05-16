//! ObjectiveAI authentication client implementation.

use crate::ctx;
use std::sync::Arc;

/// Authentication client that delegates to the ObjectiveAI HTTP API.
pub struct ObjectiveAiClient {
    /// The underlying HTTP client for ObjectiveAI API requests.
    pub client: Arc<crate::objectiveai_http::Client>,
}

impl ObjectiveAiClient {
    /// Creates a new ObjectiveAI authentication client.
    pub fn new(client: Arc<crate::objectiveai_http::Client>) -> Self {
        Self { client }
    }
}

#[async_trait::async_trait]
impl<CTXEXT> super::Client<CTXEXT> for ObjectiveAiClient
where
    CTXEXT: Send + Sync + 'static + ctx::ContextExt,
{
    async fn create_api_key<PC: crate::ctx::persistent_cache::PersistentCacheClient>(
        &self,
        ctx: ctx::Context<CTXEXT, PC>,
        request: objectiveai_sdk::auth::request::CreateApiKeyRequest,
    ) -> Result<
        objectiveai_sdk::auth::response::CreateApiKeyResponse,
        objectiveai_sdk::error::ResponseError,
    > {
        let client = self.client.with_authorization(&ctx).await;
        objectiveai_sdk::auth::create_api_key(&client, request)
            .await
            .map_err(|e| objectiveai_sdk::error::ResponseError::from(&e))
    }

    async fn create_openrouter_byok_api_key<PC: crate::ctx::persistent_cache::PersistentCacheClient>(
        &self,
        ctx: ctx::Context<CTXEXT, PC>,
        request: objectiveai_sdk::auth::request::CreateOpenRouterByokApiKeyRequest,
    ) -> Result<
        objectiveai_sdk::auth::response::CreateOpenRouterByokApiKeyResponse,
        objectiveai_sdk::error::ResponseError,
    > {
        let client = self.client.with_authorization(&ctx).await;
        objectiveai_sdk::auth::create_openrouter_byok_api_key(&client, request)
            .await
            .map_err(|e| objectiveai_sdk::error::ResponseError::from(&e))
    }

    async fn disable_api_key<PC: crate::ctx::persistent_cache::PersistentCacheClient>(
        &self,
        ctx: ctx::Context<CTXEXT, PC>,
        request: objectiveai_sdk::auth::request::DisableApiKeyRequest,
    ) -> Result<
        objectiveai_sdk::auth::response::DisableApiKeyResponse,
        objectiveai_sdk::error::ResponseError,
    > {
        let client = self.client.with_authorization(&ctx).await;
        objectiveai_sdk::auth::disable_api_key(&client, request)
            .await
            .map_err(|e| objectiveai_sdk::error::ResponseError::from(&e))
    }

    async fn delete_openrouter_byok_api_key<PC: crate::ctx::persistent_cache::PersistentCacheClient>(
        &self,
        ctx: ctx::Context<CTXEXT, PC>,
    ) -> Result<(), objectiveai_sdk::error::ResponseError> {
        let client = self.client.with_authorization(&ctx).await;
        objectiveai_sdk::auth::delete_openrouter_byok_api_key(&client)
            .await
            .map_err(|e| objectiveai_sdk::error::ResponseError::from(&e))
    }

    async fn list_api_keys<PC: crate::ctx::persistent_cache::PersistentCacheClient>(
        &self,
        ctx: ctx::Context<CTXEXT, PC>,
    ) -> Result<
        objectiveai_sdk::auth::response::ListApiKeyResponse,
        objectiveai_sdk::error::ResponseError,
    > {
        let client = self.client.with_authorization(&ctx).await;
        objectiveai_sdk::auth::list_api_keys(&client)
            .await
            .map_err(|e| objectiveai_sdk::error::ResponseError::from(&e))
    }

    async fn get_openrouter_byok_api_key<PC: crate::ctx::persistent_cache::PersistentCacheClient>(
        &self,
        ctx: ctx::Context<CTXEXT, PC>,
    ) -> Result<
        objectiveai_sdk::auth::response::GetOpenRouterByokApiKeyResponse,
        objectiveai_sdk::error::ResponseError,
    > {
        let client = self.client.with_authorization(&ctx).await;
        objectiveai_sdk::auth::get_openrouter_byok_api_key(&client)
            .await
            .map_err(|e| objectiveai_sdk::error::ResponseError::from(&e))
    }

    async fn get_credits<PC: crate::ctx::persistent_cache::PersistentCacheClient>(
        &self,
        ctx: ctx::Context<CTXEXT, PC>,
    ) -> Result<
        objectiveai_sdk::auth::response::GetCreditsResponse,
        objectiveai_sdk::error::ResponseError,
    > {
        let client = self.client.with_authorization(&ctx).await;
        objectiveai_sdk::auth::get_credits(&client)
            .await
            .map_err(|e| objectiveai_sdk::error::ResponseError::from(&e))
    }
}
