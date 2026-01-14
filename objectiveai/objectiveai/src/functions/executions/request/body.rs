use crate::{chat, functions};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionInlineProfileInlineRequestBody {
    pub function: functions::InlineFunction,
    pub profile: functions::InlineProfile,
    #[serde(flatten)]
    pub base: FunctionRemoteProfileRemoteRequestBody,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionInlineProfileRemoteRequestBody {
    pub function: functions::InlineFunction,
    #[serde(flatten)]
    pub base: FunctionRemoteProfileRemoteRequestBody,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionRemoteProfileInlineRequestBody {
    pub profile: functions::InlineProfile,
    #[serde(flatten)]
    pub base: FunctionRemoteProfileRemoteRequestBody,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionRemoteProfileRemoteRequestBody {
    // if present, retries vector completions from previous request
    #[serde(skip_serializing_if = "Option::is_none")]
    pub retry_token: Option<String>,
    // if true, vector completions use cached votes when available
    #[serde(skip_serializing_if = "Option::is_none")]
    pub from_cache: Option<bool>,
    // if true, remaining vector completion votes are RNGed
    #[serde(skip_serializing_if = "Option::is_none")]
    pub from_rng: Option<bool>,

    // reasoning config
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reasoning: Option<super::Reasoning>,

    // core config
    pub input: functions::expression::Input,
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
