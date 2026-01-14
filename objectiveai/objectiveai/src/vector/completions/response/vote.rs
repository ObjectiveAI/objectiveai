use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Vote {
    // identifiers
    pub model: String,
    pub ensemble_index: u64,
    pub flat_ensemble_index: u64,
    pub prompt_id: String, // hash of request messages
    pub tools_id: Option<String>, // hash of request tools
    pub responses_ids: Vec<String>, // hashes of request responses

    // each index corresponds to a response from the request
    pub vote: Vec<rust_decimal::Decimal>,

    // weight of the vote
    pub weight: rust_decimal::Decimal,

    // if true, vote came from a previous request
    // all fields, including weight, will be what it was in the previous request
    // from_cache will also be true
    #[serde(skip_serializing_if = "Option::is_none")]
    pub retry: Option<bool>,

    // if true, vote came from cache
    #[serde(skip_serializing_if = "Option::is_none")]
    pub from_cache: Option<bool>,

    // if true, vote was RNGed
    // from_cache and retry will be false or None
    #[serde(skip_serializing_if = "Option::is_none")]
    pub from_rng: Option<bool>,

    // internal use only
    #[serde(skip)]
    pub completion_index: Option<u64>,
}
