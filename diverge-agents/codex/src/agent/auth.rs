//! How Codex logs in: the method, as the caller states it.

use diverge_sdk::shared::containers::secrets::Mode;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::auth::{CODEX_ACCESS_TOKEN, CODEX_AUTH, OPENAI_API_KEY, OPENAI_IDENTITY_TOKEN};

/// The login method, and through it the ONE secret the agent takes.
///
/// The agent names the method and never a credential: each method
/// declares one secret, by a fixed name the template must declare,
/// and the value the caller binds to that name is what Codex logs in
/// with. The four are Codex's own ways in, as its source has them
/// (`rust-v0.163.0`, against the pinned `0.153.4`), and nothing the
/// run does with them is invented: a variable Codex reads, or a file
/// at the path Codex reads.
///
/// `OPENAI_API_KEY` in the environment is NOT one of them. Codex's
/// built-in `openai` provider has no `env_key` — the variable only
/// prefills the interactive login — so a key in the environment under
/// that name logs nothing in. `CODEX_API_KEY` is the variable the
/// credential store reads first, and `codex exec` honours it; the
/// `api_key` method is the fed `OPENAI_API_KEY` handed to Codex under
/// that name.
///
/// A ChatGPT login — the `auth_file` method with ChatGPT tokens in
/// it — speaks only to OpenAI's own backend, and a
/// [`provider`](super::Agent::provider) beside one fails as itself,
/// in Codex's words; a provider's entry is keyed by `CODEX_API_KEY`,
/// so a provider REQUIRES `api_key`, and the registration refuses
/// the pair otherwise. Tagged `method`, snake case on the wire.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "method", rename_all = "snake_case")]
pub enum Auth {
    /// An OpenAI API key. Declares `OPENAI_API_KEY`, `fixed`: a JSON
    /// string, put in the process environment as `CODEX_API_KEY`,
    /// the top-precedence credential — above `auth.json`, which is
    /// never written under this method. Bills the platform at API
    /// rates. The one method a [`provider`](super::Agent::provider)
    /// composes with.
    ApiKey,
    /// A personal access token (`at-…`) or an Agent Identity JWT.
    /// Declares `CODEX_ACCESS_TOKEN`, `fixed`: a JSON string, put in
    /// the process environment under the same name, where Codex's
    /// credential store reads it after `CODEX_API_KEY`.
    AccessToken,
    /// Codex's own `auth.json`, whole. Declares `CODEX_AUTH`,
    /// `rotating`: a JSON object of any kind the file takes — ChatGPT
    /// tokens (`auth_mode`, `tokens {id_token, access_token,
    /// refresh_token, account_id}`, `last_refresh`), an `OPENAI_API_KEY`
    /// member, a `personal_access_token`, an `agent_identity`, Bedrock
    /// credentials — written to `$CODEX_HOME/auth.json` before every
    /// loop and read back after the loop's last turn. Codex rewrites
    /// the file in place when a ChatGPT login refreshes — the three
    /// tokens and `last_refresh` rotate, five minutes before the
    /// access token's expiry, eight days after `last_refresh`, or on a
    /// `401` — and what it leaves is what the collect answers, so the
    /// next activation finds the refreshed login. Bills whatever the
    /// login is.
    AuthFile,
    /// Workload identity federation: an identity token exchanged
    /// against a federation rule. Declares `OPENAI_IDENTITY_TOKEN`,
    /// `fixed`: a JSON string, written to `$CODEX_HOME/identity-token`
    /// and named to Codex by `OPENAI_IDENTITY_TOKEN_FILE`, with
    /// `OPENAI_FEDERATION_RULE_ID` and, when given,
    /// `OPENAI_WORKLOAD_IDENTITY_CONTEXT` from the two members here.
    WorkloadIdentity {
        /// The federation rule the token is exchanged against:
        /// `OPENAI_FEDERATION_RULE_ID`.
        federation_rule_id: String,
        /// The workload's context, when the rule wants one:
        /// `OPENAI_WORKLOAD_IDENTITY_CONTEXT`. Absent leaves the
        /// variable unset.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        context: Option<String>,
    },
}

impl Auth {
    /// The one secret the method takes, by the name the template
    /// must declare it under.
    pub fn name(&self) -> &'static str {
        match self {
            Auth::ApiKey => OPENAI_API_KEY,
            Auth::AccessToken => CODEX_ACCESS_TOKEN,
            Auth::AuthFile => CODEX_AUTH,
            Auth::WorkloadIdentity { .. } => OPENAI_IDENTITY_TOKEN,
        }
    }

    /// How the method takes its secret: `auth_file` holds and hands
    /// back the file Codex rewrites; the other three read a value.
    pub fn mode(&self) -> Mode {
        match self {
            Auth::AuthFile => Mode::Rotating,
            Auth::ApiKey | Auth::AccessToken | Auth::WorkloadIdentity { .. } => Mode::Fixed,
        }
    }

    /// The method's tag, as the wire spells it.
    pub fn method(&self) -> &'static str {
        match self {
            Auth::ApiKey => "api_key",
            Auth::AccessToken => "access_token",
            Auth::AuthFile => "auth_file",
            Auth::WorkloadIdentity { .. } => "workload_identity",
        }
    }
}
