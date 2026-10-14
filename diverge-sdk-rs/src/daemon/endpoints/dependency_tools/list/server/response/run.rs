//! A dependency tool's run.

use serde::{Deserialize, Serialize};

use crate::daemon::endpoints::agents::logs::server::response::Provider;

/// The tool's container, while it runs: where, and under what id. A
/// tool has at most one run — a transient tool's for its agent's run,
/// a service's for as long as the daemon keeps it up — and none while
/// its container is down: a service held between runs has no run
/// until it runs again.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[derive(schemars::JsonSchema)]
pub struct Run {
    /// The provider the container runs on. See [`Provider`].
    pub provider: Provider,
    /// The container's id, as the provider's run answered it.
    pub id: String,
}
