//! Provider preferences for OpenRouter requests.

use crate::agent;
use serde::Serialize;

/// Provider preferences from the Agent configuration.
#[derive(Debug, Clone, PartialEq, Serialize)]
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

/// The provider request's routing preferences, field for field.
impl From<agent::Provider> for Provider {
    fn from(provider: agent::Provider) -> Self {
        Provider {
            allow_fallbacks: provider.allow_fallbacks,
            require_parameters: provider.require_parameters,
            order: provider.order,
            only: provider.only,
            ignore: provider.ignore,
            quantizations: provider
                .quantizations
                .map(|q| q.into_iter().map(Into::into).collect()),
        }
    }
}
