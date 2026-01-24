//! Request types for authentication API endpoints.
//!
//! This module contains request structures for creating, disabling, and
//! managing API keys, as well as configuring OpenRouter BYOK integration.

use serde::{Deserialize, Serialize};

/// Request to create a new API key.
///
/// # Fields
///
/// * `expires` - Optional expiration timestamp. If `None`, the key never expires.
/// * `name` - A user-provided name for identifying the key.
/// * `description` - Optional description providing additional context.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateApiKeyRequest {
    /// The expiration timestamp for the API key, or `None` for a non-expiring key.
    pub expires: Option<chrono::DateTime<chrono::Utc>>,
    /// A user-provided name to identify this API key.
    pub name: String,
    /// An optional description providing additional context about the key's purpose.
    pub description: Option<String>,
}

/// Request to disable an existing API key.
///
/// Once disabled, the API key can no longer be used for authentication.
/// This action is reversible only by creating a new key.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DisableApiKeyRequest {
    /// The API key to disable.
    pub api_key: super::ApiKey,
}

/// Request to create or update an OpenRouter BYOK (Bring Your Own Key) API key.
///
/// This allows users to provide their own OpenRouter API key for routing
/// requests through OpenRouter's model marketplace.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateOpenRouterByokApiKeyRequest {
    /// The OpenRouter API key to associate with the user's account.
    pub api_key: String,
}
