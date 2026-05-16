//! HTTP client functions for authentication endpoints.
//!
//! This module provides async functions for interacting with the ObjectiveAI
//! authentication API, including API key management, credit queries, and
//! OpenRouter BYOK configuration.
//!
//! All functions require an [`HttpClient`] instance configured with valid
//! authentication credentials.

use crate::{HttpClient, HttpError};

/// Creates a new API key.
///
/// # Arguments
///
/// * `client` - The HTTP client to use for the request.
/// * `request` - The request containing the key name, description, and expiration.
///
/// # Returns
///
/// The newly created API key with its metadata.
///
/// # Errors
///
/// Returns an [`HttpError`] if the request fails or the server returns an error.
pub async fn create_api_key(
    client: &HttpClient,
    request: super::request::CreateApiKeyRequest,
) -> Result<super::response::CreateApiKeyResponse, HttpError> {
    client
        .send_unary(reqwest::Method::POST, "auth/keys", Some(request))
        .await
}

/// Creates or updates the OpenRouter BYOK API key.
///
/// This associates an OpenRouter API key with the user's account, allowing
/// requests to be routed through OpenRouter's model marketplace.
///
/// # Arguments
///
/// * `client` - The HTTP client to use for the request.
/// * `request` - The request containing the OpenRouter API key.
///
/// # Returns
///
/// The configured OpenRouter API key.
///
/// # Errors
///
/// Returns an [`HttpError`] if the request fails or the server returns an error.
pub async fn create_openrouter_byok_api_key(
    client: &HttpClient,
    request: super::request::CreateOpenRouterByokApiKeyRequest,
) -> Result<super::response::CreateOpenRouterByokApiKeyResponse, HttpError> {
    client
        .send_unary(
            reqwest::Method::POST,
            "auth/keys/openrouter",
            Some(request),
        )
        .await
}

/// Disables an existing API key.
///
/// Once disabled, the API key can no longer be used for authentication.
/// The key's metadata is preserved with a `disabled` timestamp.
///
/// # Arguments
///
/// * `client` - The HTTP client to use for the request.
/// * `request` - The request containing the API key to disable.
///
/// # Returns
///
/// The disabled API key with updated metadata.
///
/// # Errors
///
/// Returns an [`HttpError`] if the request fails or the server returns an error.
pub async fn disable_api_key(
    client: &HttpClient,
    request: super::request::DisableApiKeyRequest,
) -> Result<super::response::DisableApiKeyResponse, HttpError> {
    client
        .send_unary(reqwest::Method::DELETE, "auth/keys", Some(request))
        .await
}

/// Deletes the OpenRouter BYOK API key.
///
/// Removes the OpenRouter API key association from the user's account.
///
/// # Arguments
///
/// * `client` - The HTTP client to use for the request.
///
/// # Errors
///
/// Returns an [`HttpError`] if the request fails or the server returns an error.
pub async fn delete_openrouter_byok_api_key(
    client: &HttpClient,
) -> Result<(), HttpError> {
    client
        .send_unary_no_response(
            reqwest::Method::DELETE,
            "auth/keys/openrouter",
            None::<String>,
        )
        .await
}

/// Lists all API keys for the authenticated user.
///
/// # Arguments
///
/// * `client` - The HTTP client to use for the request.
///
/// # Returns
///
/// A list of all API keys with their metadata and accumulated costs.
///
/// # Errors
///
/// Returns an [`HttpError`] if the request fails or the server returns an error.
pub async fn list_api_keys(
    client: &HttpClient,
) -> Result<super::response::ListApiKeyResponse, HttpError> {
    client
        .send_unary(reqwest::Method::GET, "auth/keys", None::<String>)
        .await
}

/// Retrieves the configured OpenRouter BYOK API key.
///
/// # Arguments
///
/// * `client` - The HTTP client to use for the request.
///
/// # Returns
///
/// The OpenRouter API key if configured, or `None` if not set.
///
/// # Errors
///
/// Returns an [`HttpError`] if the request fails or the server returns an error.
pub async fn get_openrouter_byok_api_key(
    client: &HttpClient,
) -> Result<super::response::GetOpenRouterByokApiKeyResponse, HttpError> {
    client
        .send_unary(
            reqwest::Method::GET,
            "auth/keys/openrouter",
            None::<String>,
        )
        .await
}

/// Retrieves the user's credit balance.
///
/// Returns the current balance, total purchased, and total used credits.
///
/// # Arguments
///
/// * `client` - The HTTP client to use for the request.
///
/// # Returns
///
/// The user's complete credit information.
///
/// # Errors
///
/// Returns an [`HttpError`] if the request fails or the server returns an error.
pub async fn get_credits(
    client: &HttpClient,
) -> Result<super::response::GetCreditsResponse, HttpError> {
    client
        .send_unary(reqwest::Method::GET, "auth/credits", None::<String>)
        .await
}
