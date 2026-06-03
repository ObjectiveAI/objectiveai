//! Response types for authentication API endpoints.
//!
//! This module contains response structures returned by the authentication
//! endpoints, including credit balances, API key metadata, and OpenRouter
//! BYOK configuration.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Response containing the user's credit balance information.
///
/// Credits are the billing unit for ObjectiveAI. This response provides
/// a complete view of the user's credit status.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "auth.GetCreditsResponse")]
pub struct GetCreditsResponse {
    /// The current available credit balance.
    #[serde(deserialize_with = "crate::serde_util::decimal")]
    #[schemars(with = "f64")]
    pub credits: rust_decimal::Decimal,
    /// The total amount of credits ever purchased.
    #[serde(deserialize_with = "crate::serde_util::decimal")]
    #[schemars(with = "f64")]
    pub total_credits_purchased: rust_decimal::Decimal,
    /// The total amount of credits consumed by API usage.
    #[serde(deserialize_with = "crate::serde_util::decimal")]
    #[schemars(with = "f64")]
    pub total_credits_used: rust_decimal::Decimal,
}

/// Response when creating a new API key.
///
/// Returns the complete API key with all associated metadata.
pub type CreateApiKeyResponse = super::ApiKeyWithMetadata;

/// Response when disabling an API key.
///
/// Returns the API key metadata, now including the `disabled` timestamp.
pub type DisableApiKeyResponse = super::ApiKeyWithMetadata;

/// Response containing a list of API keys.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "auth.ListApiKeyResponse")]
pub struct ListApiKeyResponse {
    /// The list of API keys with their metadata and usage costs.
    pub data: Vec<ListApiKeyItem>,
}

/// An API key with metadata and accumulated cost information.
///
/// This extends [`ApiKeyWithMetadata`](super::ApiKeyWithMetadata) with
/// the total cost incurred by requests using this key.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "auth.ListApiKeyItem")]
pub struct ListApiKeyItem {
    /// The API key and its metadata.
    #[serde(flatten)]
    pub inner: super::ApiKeyWithMetadata,
    /// The total cost incurred by this API key.
    #[serde(deserialize_with = "crate::serde_util::decimal")]
    #[schemars(with = "f64")]
    pub cost: rust_decimal::Decimal,
}

/// Response containing the user's OpenRouter BYOK API key.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "auth.GetOpenRouterByokApiKeyResponse")]
pub struct GetOpenRouterByokApiKeyResponse {
    /// The OpenRouter API key, or `None` if not configured.
    pub api_key: Option<String>,
}

/// Response when creating or updating an OpenRouter BYOK API key.
pub type CreateOpenRouterByokApiKeyResponse = GetOpenRouterByokApiKeyResponse;
