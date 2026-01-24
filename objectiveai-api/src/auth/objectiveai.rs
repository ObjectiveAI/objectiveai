//! ObjectiveAI authentication client implementation.

use crate::ctx;
use std::sync::Arc;

/// Authentication client that delegates to the ObjectiveAI HTTP API.
pub struct ObjectiveAiClient {
    /// The underlying HTTP client for ObjectiveAI API requests.
    pub client: Arc<objectiveai::HttpClient>,
}

impl ObjectiveAiClient {
    /// Creates a new ObjectiveAI authentication client.
    pub fn new(client: Arc<objectiveai::HttpClient>) -> Self {
        Self { client }
    }
}

#[async_trait::async_trait]
impl<CTXEXT> super::Client<CTXEXT> for ObjectiveAiClient
where
    CTXEXT: Send + Sync + 'static,
{
    async fn create_api_key(
        &self,
        _ctx: ctx::Context<CTXEXT>,
        request: objectiveai::auth::request::CreateApiKeyRequest,
    ) -> Result<
        objectiveai::auth::response::CreateApiKeyResponse,
        objectiveai::error::ResponseError,
    > {
        objectiveai::auth::create_api_key(&self.client, request)
            .await
            .map_err(|e| objectiveai::error::ResponseError::from(&e))
    }

    async fn create_openrouter_byok_api_key(
        &self,
        _ctx: ctx::Context<CTXEXT>,
        request: objectiveai::auth::request::CreateOpenRouterByokApiKeyRequest,
    ) -> Result<
        objectiveai::auth::response::CreateOpenRouterByokApiKeyResponse,
        objectiveai::error::ResponseError,
    > {
        objectiveai::auth::create_openrouter_byok_api_key(&self.client, request)
            .await
            .map_err(|e| objectiveai::error::ResponseError::from(&e))
    }

    async fn disable_api_key(
        &self,
        _ctx: ctx::Context<CTXEXT>,
        request: objectiveai::auth::request::DisableApiKeyRequest,
    ) -> Result<
        objectiveai::auth::response::DisableApiKeyResponse,
        objectiveai::error::ResponseError,
    > {
        objectiveai::auth::disable_api_key(&self.client, request)
            .await
            .map_err(|e| objectiveai::error::ResponseError::from(&e))
    }

    async fn delete_openrouter_byok_api_key(
        &self,
        _ctx: ctx::Context<CTXEXT>,
    ) -> Result<(), objectiveai::error::ResponseError> {
        objectiveai::auth::delete_openrouter_byok_api_key(&self.client)
            .await
            .map_err(|e| objectiveai::error::ResponseError::from(&e))
    }

    async fn list_api_keys(
        &self,
        _ctx: ctx::Context<CTXEXT>,
    ) -> Result<
        objectiveai::auth::response::ListApiKeyResponse,
        objectiveai::error::ResponseError,
    > {
        objectiveai::auth::list_api_keys(&self.client)
            .await
            .map_err(|e| objectiveai::error::ResponseError::from(&e))
    }

    async fn get_openrouter_byok_api_key(
        &self,
        _ctx: ctx::Context<CTXEXT>,
    ) -> Result<
        objectiveai::auth::response::GetOpenRouterByokApiKeyResponse,
        objectiveai::error::ResponseError,
    > {
        objectiveai::auth::get_openrouter_byok_api_key(&self.client)
            .await
            .map_err(|e| objectiveai::error::ResponseError::from(&e))
    }

    async fn get_credits(
        &self,
        _ctx: ctx::Context<CTXEXT>,
    ) -> Result<
        objectiveai::auth::response::GetCreditsResponse,
        objectiveai::error::ResponseError,
    > {
        objectiveai::auth::get_credits(&self.client)
            .await
            .map_err(|e| objectiveai::error::ResponseError::from(&e))
    }
}
