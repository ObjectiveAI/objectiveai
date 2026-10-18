//! The Codex agent.

use diverge_sdk::shared::containers::dependencies::Template;
use diverge_sdk::shared::containers::secrets::Declaration;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::{Auth, Effort, Provider, Summary, Verbosity, WebSearch};

/// An agent running against Codex.
///
/// Few knobs, and not because anything is missing: Codex decides its
/// own sampling, its own tools and its own loop, and what it exposes
/// to a caller is the model, how hard it reasons and how it speaks,
/// whether it may search the web, where it sends its requests, and
/// how it logs in — the method only, never the credential, which is
/// the one secret the method declares (see [`Auth`]). Each field is
/// one config key or flag, named in its doc; every `Option` absent
/// leaves that key unset, so Codex applies its own default for the
/// model — except [`web_search`](Self::web_search), where absent is
/// off, because nothing is on by omission.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Agent {
    /// The model to run: `model` (`--model`), in the endpoint's own
    /// naming.
    pub model: String,
    /// How much effort the model spends reasoning:
    /// `model_reasoning_effort`. See [`Effort`].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub effort: Option<Effort>,
    /// How much of its reasoning the model says out loud:
    /// `model_reasoning_summary`. What the stream's reasoning chunks
    /// carry. See [`Summary`].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reasoning_summary: Option<Summary>,
    /// How long the model's answers run: `model_verbosity`. See
    /// [`Verbosity`].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub verbosity: Option<Verbosity>,
    /// Whether, and how, the model may search the web: `web_search`.
    /// A capability of Codex itself rather than a tool of the
    /// caller's — Codex performs the search and never calls out.
    /// ABSENT IS `disabled`. See [`WebSearch`].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub web_search: Option<WebSearch>,
    /// Where requests go instead of OpenAI: a `model_providers` entry
    /// the harness writes and selects, keyed by `CODEX_API_KEY` — so
    /// it requires the `api_key` login, and the registration refuses
    /// it beside another. Absent is OpenAI's own endpoint. See
    /// [`Provider`].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provider: Option<Provider>,
    /// How Codex logs in: one of its own methods, tagged `method`,
    /// each declaring the one secret the proxy feeds. REQUIRED — no
    /// method is the default, because every one bills someone. See
    /// [`Auth`].
    pub auth: Auth,
    /// The tool containers this agent depends on, each a dependency
    /// tool template the caller deploys and serves to it as an MCP
    /// server, exactly as the SDK's [`Template`] states one: that
    /// type's JSON Schema is what `GET /schema` states for this member,
    /// and this list, unchanged, is what the registration answers —
    /// one schema, from the one type, at both. Absent is none.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub dependencies: Vec<Template>,
}

impl Agent {
    /// The secrets this agent takes, as the registration answers
    /// them: the one name its login method declares, in the mode the
    /// method takes it.
    pub fn declaration(&self) -> Declaration {
        let mut declaration = Declaration::new();
        declaration.insert(self.auth.name().to_string(), self.auth.mode());
        declaration
    }

    /// The one rule the registration enforces across fields: a
    /// [`provider`](Self::provider) is keyed by `CODEX_API_KEY`, so
    /// it composes with the `api_key` method and no other — a ChatGPT
    /// login speaks only to OpenAI's backend, and a token or an
    /// identity exchange never reaches a custom endpoint's `env_key`.
    pub fn check(&self) -> Result<(), Mismatch> {
        match (&self.provider, &self.auth) {
            (Some(_), Auth::ApiKey) | (None, _) => Ok(()),
            (Some(_), auth) => Err(Mismatch::Provider(auth.method())),
        }
    }
}

/// Two fields that do not compose.
#[derive(Debug)]
pub enum Mismatch {
    /// A provider beside a login method other than `api_key`; the
    /// method's tag is carried.
    Provider(&'static str),
}

impl std::fmt::Display for Mismatch {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Mismatch::Provider(method) => write!(
                f,
                "a provider is keyed by CODEX_API_KEY and requires the api_key method, not {method}"
            ),
        }
    }
}

impl std::error::Error for Mismatch {}

impl Mismatch {
    /// The failure as JSON, the shape every refusal has.
    pub fn message(&self) -> serde_json::Value {
        serde_json::json!({
            "kind": "auth",
            "error": self.to_string(),
        })
    }
}
