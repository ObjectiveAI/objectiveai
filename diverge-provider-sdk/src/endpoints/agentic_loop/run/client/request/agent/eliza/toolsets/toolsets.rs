//! The toolsets — every switch in one place.

use serde::{Deserialize, Serialize};

/// Per-plugin switches over Eliza's local capabilities, narrowed to
/// what a request can make work in the container. Each boolean is
/// an `Option<bool>` where absent is off — the same as `false` — so
/// a request names only what it turns on.
///
/// These are the agent's LOCAL capabilities. The caller's MCP tools
/// are not in this vocabulary and never need to be — the harness
/// always wires them in. What is deliberately ABSENT, and why, is
/// [the module](super)'s to say.
#[derive(
    Debug, Clone, Default, PartialEq, Eq, Hash, Serialize, Deserialize,
)]
pub struct Toolsets {
    /// `plugin-coding-tools`: Claude-Code-style Read, Write, Edit,
    /// Bash, Grep, Glob and friends, in the run's workspace. Nothing
    /// to configure — the container is the sandbox.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub coding_tools: Option<bool>,
    /// `plugin-browser`: the BROWSER action. Headless here, on the
    /// plugin's jsdom fallback — no embedded browser, no companion
    /// bridge.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub browser: Option<bool>,
    /// `plugin-documents` and the documents feature: ingestion of
    /// the files the request mounted at the documents path into
    /// searchable document and fragment memories (content-hash
    /// deduplicated, so re-mounting each run is free), and search
    /// over them. Vectors need an [`Embedding`](super::super::Embedding);
    /// without one the search is by keyword.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub documents: Option<bool>,
    /// The built-in GENERATE_MEDIA action. Its images come from the
    /// provider's image tier — `plugin-openai` registers one against
    /// the same base URL — so an endpoint without an image model
    /// fails the action as itself.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub generate_media: Option<bool>,
    /// `plugin-web-search`, keyed. See
    /// [`web_search`](super::web_search).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub web_search: Option<super::web_search::Toolset>,
}
