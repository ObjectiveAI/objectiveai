use crate::chat;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VectorCompletionCreateParams {
    // if present, reuses votes from previous request
    #[serde(skip_serializing_if = "Option::is_none")]
    pub retry: Option<String>,
    // if true, uses cached votes when available
    #[serde(skip_serializing_if = "Option::is_none")]
    pub from_cache: Option<bool>,
    // if true, remaining votes are RNGed
    #[serde(skip_serializing_if = "Option::is_none")]
    pub from_rng: Option<bool>,

    // core config
    pub messages: Vec<chat::completions::request::Message>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider: Option<chat::completions::request::Provider>,
    pub ensemble: super::Ensemble,
    pub profile: Vec<rust_decimal::Decimal>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub seed: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<chat::completions::request::Tool>>, // readonly
    pub responses: Vec<chat::completions::request::RichContent>,

    // retry config
    #[serde(skip_serializing_if = "Option::is_none")]
    pub backoff_max_elapsed_time: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub first_chunk_timeout: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub other_chunk_timeout: Option<u64>,
}
