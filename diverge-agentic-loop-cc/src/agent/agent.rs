//! The Claude Code agent.

use diverge_sdk::shared::containers::dependencies::Template;
use diverge_sdk::shared::containers::secrets::Declaration;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::{Effort, Tools};

/// An agent running against Claude Code.
///
/// Far fewer knobs than OpenRouter's, and not because anything is
/// missing: Claude Code is an agent harness in its own right, so
/// the sampling decisions OpenRouter exposes are made inside it.
/// What it does expose is its tools: [`tools`](Self::tools) is the
/// exact set of built-ins the model is given, every switch stated,
/// passed to Claude Code as `--tools`;
/// [`dependencies`](Self::dependencies) is the other kind, the tool
/// containers the agent asks the caller to run for it.
///
/// # Secrets
///
/// None: Claude Code logs in by itself, and no argument names a
/// credential. The template this agent is made from need declare no
/// name, and a name it does declare is one the program never asks
/// for.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, Default)]
pub struct Agent {
    /// The model to run.
    pub model: String,
    /// The switchable built-in tools — each absent or `false` is
    /// off, and the whole structure absent is every tool off. The
    /// MCP tools and `Skill` are on regardless; see [`Tools`].
    #[serde(default)]
    pub tools: Tools,
    /// Whether to think before answering.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub thinking: Option<bool>,
    /// How much effort to spend thinking. Meaningful only with
    /// [`thinking`](Self::thinking) on.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub effort: Option<Effort>,
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
    /// them: none, whatever the arguments.
    pub fn declaration(&self) -> Declaration {
        Declaration::new()
    }
}
