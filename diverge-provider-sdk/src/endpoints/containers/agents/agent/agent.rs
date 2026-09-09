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
/// not overlap: Codex's knobs mean nothing to a Python agent, which
/// has no sampling parameters at all. A union of every provider's
/// knobs would be a struct where most fields are always absent, and
/// would leave a provider to discover at runtime that it was handed
/// something it cannot honour.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Agent {
    /// See [`codex::Agent`](super::codex::Agent).
    Codex(super::codex::Agent),
    /// See [`python::Agent`](super::python::Agent).
    Python(super::python::Agent),
}
