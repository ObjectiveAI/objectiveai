//! Provider preferences for OpenRouter requests.

use serde::{Deserialize, Serialize};

/// Provider preferences merged from request and Ensemble LLM configuration.
///
/// Some fields come from the Ensemble LLM (allow_fallbacks, require_parameters, etc.)
/// while others come from the request (data_collection, zdr, sort, etc.).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Provider {
    /// Whether to allow fallback to other providers. From Ensemble LLM.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allow_fallbacks: Option<bool>,
    /// Whether to require all parameters. From Ensemble LLM.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub require_parameters: Option<bool>,
    /// Data collection preferences. From request.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data_collection: Option<objectiveai::chat::completions::request::ProviderDataCollection>,
    /// Zero Data Retention preference. From request.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub zdr: Option<bool>,
    /// Provider order preference. From Ensemble LLM.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub order: Option<Vec<String>>,
    /// Only use these providers. From Ensemble LLM.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub only: Option<Vec<String>>,
    /// Ignore these providers. From Ensemble LLM.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ignore: Option<Vec<String>>,
    /// Allowed quantizations. From Ensemble LLM.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub quantizations: Option<Vec<objectiveai::ensemble_llm::ProviderQuantization>>,
    /// Provider sort preference. From request.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sort: Option<objectiveai::chat::completions::request::ProviderSort>,
    /// Maximum price constraints. From request.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_price: Option<objectiveai::chat::completions::request::ProviderMaxPrice>,
    /// Preferred minimum throughput. From request.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub preferred_min_throughput: Option<f64>,
    /// Preferred maximum latency. From request.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub preferred_max_latency: Option<f64>,
    /// Hard minimum throughput requirement. From request.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_throughput: Option<f64>,
    /// Hard maximum latency requirement. From request.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_latency: Option<f64>,
}

impl Provider {
    /// Returns true if all fields are None.
    pub fn is_empty(&self) -> bool {
        self.allow_fallbacks.is_none()
            && self.require_parameters.is_none()
            && self.data_collection.is_none()
            && self.zdr.is_none()
            && self.order.is_none()
            && self.only.is_none()
            && self.ignore.is_none()
            && self.quantizations.is_none()
            && self.sort.is_none()
            && self.max_price.is_none()
            && self.preferred_min_throughput.is_none()
            && self.preferred_max_latency.is_none()
            && self.min_throughput.is_none()
            && self.max_latency.is_none()
    }

    /// Creates a new Provider by merging request and Ensemble LLM preferences.
    ///
    /// Returns None if both inputs are None or if the merged result is empty.
    pub fn new(
        request: Option<objectiveai::chat::completions::request::Provider>,
        ensemble_llm: Option<&objectiveai::ensemble_llm::Provider>,
    ) -> Option<Self> {
        let provider = match (request, ensemble_llm) {
            (
                Some(objectiveai::chat::completions::request::Provider {
                    data_collection,
                    zdr,
                    sort,
                    max_price,
                    preferred_min_throughput,
                    preferred_max_latency,
                    min_throughput,
                    max_latency,
                }),
                Some(objectiveai::ensemble_llm::Provider {
                    allow_fallbacks,
                    require_parameters,
                    order,
                    only,
                    ignore,
                    quantizations,
                }),
            ) => Self {
                allow_fallbacks: *allow_fallbacks,
                require_parameters: *require_parameters,
                data_collection,
                zdr,
                order: order.clone(),
                only: only.clone(),
                ignore: ignore.clone(),
                quantizations: quantizations.clone(),
                sort,
                max_price,
                preferred_min_throughput,
                preferred_max_latency,
                min_throughput,
                max_latency,
            },
            (
                Some(objectiveai::chat::completions::request::Provider {
                    data_collection,
                    zdr,
                    sort,
                    max_price,
                    preferred_min_throughput,
                    preferred_max_latency,
                    min_throughput,
                    max_latency,
                }),
                None,
            ) => Self {
                allow_fallbacks: None,
                require_parameters: None,
                data_collection,
                zdr,
                order: None,
                only: None,
                ignore: None,
                quantizations: None,
                sort,
                max_price,
                preferred_min_throughput,
                preferred_max_latency,
                min_throughput,
                max_latency,
            },
            (
                None,
                Some(objectiveai::ensemble_llm::Provider {
                    allow_fallbacks,
                    require_parameters,
                    order,
                    only,
                    ignore,
                    quantizations,
                }),
            ) => Self {
                allow_fallbacks: *allow_fallbacks,
                require_parameters: *require_parameters,
                data_collection: None,
                zdr: None,
                order: order.clone(),
                only: only.clone(),
                ignore: ignore.clone(),
                quantizations: quantizations.clone(),
                sort: None,
                max_price: None,
                preferred_min_throughput: None,
                preferred_max_latency: None,
                min_throughput: None,
                max_latency: None,
            },
            (None, None) => return None,
        };
        if provider.is_empty() {
            None
        } else {
            Some(provider)
        }
    }
}
