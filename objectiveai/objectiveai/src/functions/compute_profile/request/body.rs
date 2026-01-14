use crate::{chat, functions, vector};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionInlineRequestBody {
    pub function: functions::InlineFunction,
    #[serde(flatten)]
    pub base: FunctionRemoteRequestBody,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionRemoteRequestBody {
    // if present, retries vector completions from previous request
    #[serde(skip_serializing_if = "Option::is_none")]
    pub retry_token: Option<String>,
    // if true, vector completions use cached votes when available
    #[serde(skip_serializing_if = "Option::is_none")]
    pub from_cache: Option<bool>,
    // if true, remaining vector completion votes are RNGed
    #[serde(skip_serializing_if = "Option::is_none")]
    pub from_rng: Option<bool>,

    // core config
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_retries: Option<u64>,
    pub n: u64,
    pub dataset: Vec<super::DatasetItem>,
    pub ensemble: vector::completions::request::Ensemble,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider: Option<chat::completions::request::Provider>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub seed: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream: Option<bool>,

    // retry config
    #[serde(skip_serializing_if = "Option::is_none")]
    pub backoff_max_elapsed_time: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub first_chunk_timeout: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub other_chunk_timeout: Option<u64>,
}
