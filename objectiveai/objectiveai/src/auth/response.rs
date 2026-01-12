use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetCreditsResponse {
    pub credits: rust_decimal::Decimal,
    pub total_credits_purchased: rust_decimal::Decimal,
    pub total_credits_used: rust_decimal::Decimal,
}

pub type CreateApiKeyResponse = super::ApiKeyWithMetadata;

pub type DisableApiKeyResponse = super::ApiKeyWithMetadata;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListApiKeyItem {
    #[serde(flatten)]
    pub inner: super::ApiKeyWithMetadata,
    pub cost: rust_decimal::Decimal,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetOpenRouterApiKeyResponse {
    pub api_key: Option<String>,
}

pub type SetOpenRouterApiKeyResponse = GetOpenRouterApiKeyResponse;
