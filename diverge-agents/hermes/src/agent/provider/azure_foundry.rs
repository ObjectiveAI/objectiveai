//! Microsoft Azure Foundry.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Microsoft Azure Foundry — a caller-supplied deployment, so the
/// endpoint is as much the credential as the key is.
///
/// APPLICATION: the harness sets `AZURE_FOUNDRY_API_KEY` to the
/// secret's value and `AZURE_FOUNDRY_BASE_URL` to
/// [`base_url`](Self::base_url) in the gateway's process environment
/// before Hermes starts. Both are required: Hermes raises a hard
/// auth error on a missing base URL. The Entra-ID mode is
/// deliberately absent — ambient Azure identity is not an argument.
///
/// SECRET: `AZURE_FOUNDRY_API_KEY`, fixed — the API key, under that
/// name.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Provider {
    /// The discriminator. Always `azure-foundry`.
    pub provider: AzureFoundry,
    /// The deployment's endpoint, applied as
    /// `AZURE_FOUNDRY_BASE_URL`.
    pub base_url: String,
}

/// [`Provider`]'s discriminator.
///
/// One variant, and the reason the [union](super::Provider) can be
/// untagged: no other provider's arguments can produce this value.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "kebab-case")]
pub enum AzureFoundry {
    #[default]
    AzureFoundry,
}
