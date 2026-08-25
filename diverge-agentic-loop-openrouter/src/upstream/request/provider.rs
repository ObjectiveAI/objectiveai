//! Provider preferences for OpenRouter requests.

use serde::{Deserialize, Serialize};

/// Provider preferences from the Agent configuration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Provider {
    /// Whether to allow fallback to other providers. From Agent.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allow_fallbacks: Option<bool>,
    /// Whether to require all parameters. From Agent.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub require_parameters: Option<bool>,
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
}
