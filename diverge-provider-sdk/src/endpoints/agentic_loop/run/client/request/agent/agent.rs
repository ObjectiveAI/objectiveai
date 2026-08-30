//! The agent — one upstream's parameters.

use serde::{Deserialize, Serialize};

/// The agent a provider is asked to run.
///
/// Untagged, discriminated by each variant's own `upstream` constant —
/// the same discipline the response chunks use for `type`. serde has
/// no tag of its own to read, so an agent goes on the wire as its
/// parameters rather than as a wrapper around them.
///
/// One variant per upstream because the parameter sets genuinely do
/// not overlap: `logit_bias` means nothing to Claude Code,
/// `thinking` means nothing to OpenRouter, and a Python agent has
/// no sampling parameters at all. A union of every provider's
/// knobs would be a struct where most fields are always absent, and
/// would leave a provider to discover at runtime that it was handed
/// something it cannot honour.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Agent {
    /// See [`openrouter::Agent`](super::openrouter::Agent).
    Openrouter(super::openrouter::Agent),
    /// See [`claude_code::Agent`](super::claude_code::Agent).
    ClaudeCode(super::claude_code::Agent),
    /// See [`codex::Agent`](super::codex::Agent).
    Codex(super::codex::Agent),
    /// See [`hermes::Agent`](super::hermes::Agent).
    Hermes(super::hermes::Agent),
    /// See [`python::Agent`](super::python::Agent).
    Python(super::python::Agent),
}
