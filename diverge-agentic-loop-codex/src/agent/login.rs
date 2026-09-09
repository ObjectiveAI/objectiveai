//! How Codex logs in.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// How Codex logs in: `forced_login_method`, and with it the vault
/// key the run reads. The value names the method only — the
/// credential itself never appears in the agent value, in a row, or
/// in the schema.
///
/// - `api_key`: the vault's `OPENAI_API_KEY`, a STATIC secret, read
///   once per run and handed to Codex as the `env_key` of its
///   provider. A vault without it refuses the run.
/// - `chatgpt`: the vault's well-known `OPENAI_CODEX_OAUTH` — the
///   SDK's [`vault::keys::OPENAI_CODEX_OAUTH`](diverge_provider_sdk::shared::containers::vault::keys::OPENAI_CODEX_OAUTH),
///   a ROTATING login shared with every image that speaks it —
///   rendered as `$CODEX_HOME/auth.json`. Codex refreshes the tokens
///   during use, so the run owes the cycle: lock, get, write, run,
///   read back, set, unlock.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum Login {
    /// An API key: `forced_login_method = "api"`.
    ApiKey,
    /// A ChatGPT account: `forced_login_method = "chatgpt"`.
    Chatgpt,
}
