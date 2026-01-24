//! Authentication client trait definition.

use crate::ctx;

/// Trait for authentication operations.
///
/// Provides methods for managing API keys, BYOK OpenRouter keys, and credits.
/// Generic over `CTXEXT` to allow custom context extensions.
#[async_trait::async_trait]
pub trait Client<CTXEXT> {
    /// Creates a new API key for the authenticated user.
    async fn create_api_key(
        &self,
        ctx: ctx::Context<CTXEXT>,
        request: objectiveai::auth::request::CreateApiKeyRequest,
    ) -> Result<
        objectiveai::auth::response::CreateApiKeyResponse,
        objectiveai::error::ResponseError,
    >;

    /// Sets the user's BYOK (Bring Your Own Key) OpenRouter API key.
    async fn create_openrouter_byok_api_key(
        &self,
        ctx: ctx::Context<CTXEXT>,
        request: objectiveai::auth::request::CreateOpenRouterByokApiKeyRequest,
    ) -> Result<
        objectiveai::auth::response::CreateOpenRouterByokApiKeyResponse,
        objectiveai::error::ResponseError,
    >;

    /// Disables an existing API key.
    async fn disable_api_key(
        &self,
        ctx: ctx::Context<CTXEXT>,
        request: objectiveai::auth::request::DisableApiKeyRequest,
    ) -> Result<
        objectiveai::auth::response::DisableApiKeyResponse,
        objectiveai::error::ResponseError,
    >;

    /// Deletes the user's BYOK OpenRouter API key.
    async fn delete_openrouter_byok_api_key(
        &self,
        ctx: ctx::Context<CTXEXT>,
    ) -> Result<(), objectiveai::error::ResponseError>;

    /// Lists all API keys for the authenticated user.
    async fn list_api_keys(
        &self,
        ctx: ctx::Context<CTXEXT>,
    ) -> Result<
        objectiveai::auth::response::ListApiKeyResponse,
        objectiveai::error::ResponseError,
    >;

    /// Retrieves the user's BYOK OpenRouter API key.
    async fn get_openrouter_byok_api_key(
        &self,
        ctx: ctx::Context<CTXEXT>,
    ) -> Result<
        objectiveai::auth::response::GetOpenRouterByokApiKeyResponse,
        objectiveai::error::ResponseError,
    >;

    /// Retrieves the user's available credit balance.
    async fn get_credits(
        &self,
        ctx: ctx::Context<CTXEXT>,
    ) -> Result<
        objectiveai::auth::response::GetCreditsResponse,
        objectiveai::error::ResponseError,
    >;
}
