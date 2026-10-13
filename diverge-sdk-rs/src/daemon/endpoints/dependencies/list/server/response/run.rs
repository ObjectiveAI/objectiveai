//! One tool running from a dependency template.

use serde::{Deserialize, Serialize};

use crate::daemon::endpoints::agents::logs::server::response::Provider;
use crate::daemon::key;

/// One tool running from a dependency template now: whose it is,
/// which with the template names it once and for all — the
/// [`key::Tool::Dependency`](crate::daemon::key::Tool::Dependency)
/// the daemon reports it as elsewhere — and where it runs. A
/// transient tool's owner is the agent it was deployed for; a
/// service's is whatever its template's `scope` names. A tool that
/// does not run is no run: a service held between runs is listed by
/// nothing until it runs again.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Run {
    /// Whose the tool is: see [`key::Owner`].
    pub owner: key::Owner,
    /// The provider the container runs on. See [`Provider`].
    pub provider: Provider,
    /// The container's id, as the provider's run answered it.
    pub id: String,
}
