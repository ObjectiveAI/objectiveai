//! Provider preferences for OpenRouter requests.

use serde::{Deserialize, Serialize};

/// Provider preferences merged from request and Agent configuration.
///
/// Some fields come from the Agent (allow_fallbacks, require_parameters, etc.)
/// while others come from the request (data_collection, zdr, sort, etc.).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Provider {
    /// Whether to allow fallback to other providers. From Agent.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allow_fallbacks: Option<bool>,
    /// Whether to require all parameters. From Agent.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub require_parameters: Option<bool>,
    /// Data collection preferences. From request.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data_collection: Option<super::ProviderDataCollection>,
    /// Zero Data Retention preference. From request.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub zdr: Option<bool>,
    /// Provider order preference. From Agent.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub order: Option<Vec<String>>,
    /// Only use these providers. From Agent.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub only: Option<Vec<String>>,
    /// Ignore these providers. From Agent.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ignore: Option<Vec<String>>,
    /// Allowed quantizations. From Agent.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub quantizations: Option<Vec<super::ProviderQuantization>>,
    /// Provider sort preference. From request.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sort: Option<super::ProviderSort>,
    /// Maximum price constraints. From request.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_price: Option<super::ProviderMaxPrice>,
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
