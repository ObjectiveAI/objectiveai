//! What the filesystem preparation hands back.

use std::collections::BTreeMap;

/// The filesystem is ready; this is what the run still needs to
/// carry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Prepared {
    /// Whether a continuation landed: `true`, the run resumes the
    /// session on disk; `false`, the fresh start.
    pub resumed: bool,
    /// The environment to set on the `hermes gateway` process — the
    /// harness's variables, and only those.
    pub env: BTreeMap<String, String>,
    /// The bearer the API server was given, for the run driver to
    /// present on every request to it. Also in [`env`](Self::env)
    /// as `API_SERVER_KEY`.
    pub api_server_key: String,
}
