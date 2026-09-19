//! The Codex agent.

use diverge_provider_sdk::shared::containers::tools::Tool;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::{Effort, Provider, Summary, Verbosity, WebSearch};

/// An agent running against Codex.
///
/// Few knobs, and not because anything is missing: Codex decides its
/// own sampling, its own tools and its own loop, and what it exposes
/// to a caller is the model, how hard it reasons and how it speaks,
/// whether it may search the web, and where it sends its requests.
/// How it logs in is not the agent's to say: the login is a mount or
/// the vault's (see [the module](super)). Each field is one config
/// key or flag, named in its
/// doc; every `Option` absent leaves that key unset, so Codex applies
/// its own default for the model — except [`web_search`](Self::web_search),
/// where absent is off, because nothing is on by omission.
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
    /// the harness writes and selects, its key the `OPENAI_API_KEY`
    /// the run found. Absent is OpenAI's own endpoint. See
    /// [`Provider`].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provider: Option<Provider>,
    /// The tool containers this agent depends on, each one the caller
    /// runs and serves to it as an MCP server, in the form the
    /// provider's wire defines. Passed back whole as the registration's
    /// answer, which is how the caller learns of them. Absent is none.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub mcp_tools: Vec<Tool>,
}
