//! CommandCode's Anthropic-dialect route.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// CommandCode's Anthropic-dialect route.
///
/// APPLICATION: the harness sets `COMMANDCODE_API_KEY` to
/// [`api_key`](Self::api_key) in the gateway's process environment
/// before Hermes starts.
/// (The same key as [`commandcode`](super::commandcode::Provider)
/// — one credential, two wires.)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Provider {
    /// The discriminator. Always `commandcode-anthropic`.
    pub provider: CommandcodeAnthropic,
    /// The API key, applied as `COMMANDCODE_API_KEY`.
    pub api_key: String,
}

/// [`Provider`]'s discriminator.
///
/// One variant, and the reason the [union](super::Provider) can be
/// untagged: no other provider's arguments can produce this value.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "kebab-case")]
pub enum CommandcodeAnthropic {
    #[default]
    CommandcodeAnthropic,
}
