//! What the filesystem preparation hands back.

use std::collections::BTreeMap;

/// The filesystem is ready; this is what the run still needs to
/// carry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Prepared {
    /// The session to resume — the lineage's tip in the delivered
    /// database — or `None`, the fresh start.
    pub session: Option<String>,
    /// The environment to set on the `hermes gateway` process — the
    /// harness's variables, and only those.
    pub env: BTreeMap<String, String>,
    /// The bearer the API server was given, for the run driver to
    /// present on every request to it. Also in [`env`](Self::env)
    /// as `API_SERVER_KEY`.
    pub api_server_key: String,
}
