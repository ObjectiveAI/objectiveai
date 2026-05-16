//! Authentication client trait definition.

use crate::ctx;

/// Trait for authentication operations.
///
/// Provides methods for managing API keys, BYOK OpenRouter keys, and credits.
/// Generic over `CTXEXT` to allow custom context extensions.
#[async_trait::async_trait]
pub trait Client<CTXEXT> {
    /// Creates a new API key for the authenticated user.
    async fn create_api_key<PC: crate::ctx::persistent_cache::PersistentCacheClient>(
        &self,
        ctx: ctx::Context<CTXEXT, PC>,
        request: objectiveai_sdk::auth::request::CreateApiKeyRequest,
    ) -> Result<
        objectiveai_sdk::auth::response::CreateApiKeyResponse,
        objectiveai_sdk::error::ResponseError,
    >;

    /// Sets the user's BYOK (Bring Your Own Key) OpenRouter API key.
    async fn create_openrouter_byok_api_key<PC: crate::ctx::persistent_cache::PersistentCacheClient>(
        &self,
        ctx: ctx::Context<CTXEXT, PC>,
        request: objectiveai_sdk::auth::request::CreateOpenRouterByokApiKeyRequest,
    ) -> Result<
        objectiveai_sdk::auth::response::CreateOpenRouterByokApiKeyResponse,
        objectiveai_sdk::error::ResponseError,
    >;

    /// Disables an existing API key.
    async fn disable_api_key<PC: crate::ctx::persistent_cache::PersistentCacheClient>(
        &self,
        ctx: ctx::Context<CTXEXT, PC>,
        request: objectiveai_sdk::auth::request::DisableApiKeyRequest,
    ) -> Result<
        objectiveai_sdk::auth::response::DisableApiKeyResponse,
        objectiveai_sdk::error::ResponseError,
    >;

    /// Deletes the user's BYOK OpenRouter API key.
    async fn delete_openrouter_byok_api_key<PC: crate::ctx::persistent_cache::PersistentCacheClient>(
        &self,
        ctx: ctx::Context<CTXEXT, PC>,
    ) -> Result<(), objectiveai_sdk::error::ResponseError>;

    /// Lists all API keys for the authenticated user.
    async fn list_api_keys<PC: crate::ctx::persistent_cache::PersistentCacheClient>(
        &self,
        ctx: ctx::Context<CTXEXT, PC>,
    ) -> Result<
        objectiveai_sdk::auth::response::ListApiKeyResponse,
        objectiveai_sdk::error::ResponseError,
    >;

    /// Retrieves the user's BYOK OpenRouter API key.
    async fn get_openrouter_byok_api_key<PC: crate::ctx::persistent_cache::PersistentCacheClient>(
        &self,
        ctx: ctx::Context<CTXEXT, PC>,
    ) -> Result<
        objectiveai_sdk::auth::response::GetOpenRouterByokApiKeyResponse,
        objectiveai_sdk::error::ResponseError,
    >;

    /// Retrieves the user's available credit balance.
    async fn get_credits<PC: crate::ctx::persistent_cache::PersistentCacheClient>(
        &self,
        ctx: ctx::Context<CTXEXT, PC>,
    ) -> Result<
        objectiveai_sdk::auth::response::GetCreditsResponse,
        objectiveai_sdk::error::ResponseError,
    >;
}
