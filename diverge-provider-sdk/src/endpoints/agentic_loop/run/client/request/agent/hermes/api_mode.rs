//! Which API dialect an endpoint speaks.

use serde::{Deserialize, Serialize};

/// The five wire dialects Hermes can drive an endpoint with — the
/// vocabulary its runtime accepts, closed at the pin.
///
/// Usually unsaid: every bundled provider profile carries its own
/// dialect. This matters for [`custom`](super::Provider::Custom)
/// endpoints, and for overriding a profile that guesses wrong.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum ApiMode {
    /// OpenAI-compatible chat completions.
    #[default]
    ChatCompletions,
    /// The OpenAI Responses dialect (Codex's).
    CodexResponses,
    /// Anthropic's Messages API.
    AnthropicMessages,
    /// AWS Bedrock Converse.
    BedrockConverse,
    /// The Codex app-server runtime.
    CodexAppServer,
}
