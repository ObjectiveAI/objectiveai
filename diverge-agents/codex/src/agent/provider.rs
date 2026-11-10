//! Where requests go.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// An endpoint other than OpenAI's: one `model_providers` entry the
/// harness writes as `model_providers.diverge` and selects with
/// `model_provider = "diverge"`. The entry's `env_key` is
/// `CODEX_API_KEY` — the variable the `api_key` login method hands
/// the fed `OPENAI_API_KEY` to Codex under — and its `wire_api` is
/// `responses`, the one value the configuration reference names, so
/// the endpoint must speak the Responses API, which the Diverge relay
/// and every upstream this protocol routes do.
///
/// Absent, Codex's own `openai` provider is used, with whatever
/// login the agent's [`auth`](super::Agent::auth) names. Present, the
/// entry REQUIRES the `api_key` method: a ChatGPT login speaks only
/// to OpenAI's own backend, and no other method fills an `env_key`,
/// so the registration refuses a provider beside any other method
/// (`400`, `{"kind":"auth"}`).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
pub struct Provider {
    /// The base URL, `/v1` included: `model_providers.diverge.base_url`.
    pub base_url: String,
}
