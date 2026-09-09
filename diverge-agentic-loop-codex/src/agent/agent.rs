//! The Codex agent.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::{Effort, Login, Provider, Summary, Verbosity, WebSearch};

/// An agent running against Codex.
///
/// Few knobs, and not because anything is missing: Codex decides its
/// own sampling, its own tools and its own loop, and what it exposes
/// to a caller is the model, how hard it reasons and how it speaks,
/// whether it may search the web, how it logs in, and where it sends
/// its requests. Each field is one config key or flag, named in its
/// doc; every `Option` absent leaves that key unset, so Codex applies
/// its own default for the model — except [`web_search`](Self::web_search),
/// where absent is off, because nothing is on by omission.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
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
    /// How Codex logs in, and so which vault key the run reads:
    /// `forced_login_method`. Absent is an API key. See [`Login`].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub login: Option<Login>,
    /// Where requests go instead of OpenAI: a `model_providers` entry
    /// the harness writes and selects. Needs an API-key
    /// [`login`](Self::login); with a ChatGPT login it is a
    /// contradiction the run refuses. Absent is OpenAI's own
    /// endpoint. See [`Provider`].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provider: Option<Provider>,
}
