//! Where requests go.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// An endpoint other than OpenAI's: one `model_providers` entry the
/// harness writes as `model_providers.diverge` and selects with
/// `model_provider = "diverge"`. The entry's `env_key` is
/// `OPENAI_API_KEY`, the vault's, and its `wire_api` is `responses`,
/// the one value the configuration reference names — so the endpoint
/// must speak the Responses API, which the Diverge relay and every
/// upstream this protocol routes do.
///
/// Absent, Codex's own `openai` provider is used, and the login may
/// be either kind. Present, the login must be an API key: a ChatGPT
/// login speaks only to OpenAI's own backend, and a request naming
/// both is a contradiction the run refuses.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
pub struct Provider {
    /// The base URL, `/v1` included: `model_providers.diverge.base_url`.
    pub base_url: String,
}
