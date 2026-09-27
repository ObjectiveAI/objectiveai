//! One tool, as the daemon holds it.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::daemon::endpoints::agents::create::client::request::Image;
use crate::daemon::endpoints::agents::logs::server::response::Provider;

/// One tool of the caller's: what the daemon knows of it.
///
/// Everything here the daemon holds for the tool's life, so a list
/// costs no more than the tools it names. What it was made from
/// beyond the image — its limits, its mounts, its arguments — is the
/// create's, and is not repeated here.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Tool {
    /// The name, as its create gave it.
    pub name: String,
    /// The image it runs: the name and the digest the create named.
    pub image: Image,
    /// When the create made it.
    pub created: DateTime<Utc>,
    /// Whether the tool container is running now: an agent it is
    /// attached to is active.
    pub active: bool,
    /// When that last changed: the time the container started, if it
    /// runs; the time it last stopped, if it does not; absent for a
    /// tool that has never run.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_active: Option<DateTime<Utc>>,
    /// The provider the container runs on, if it runs, or last ran
    /// on; absent for a tool that has never run. See [`Provider`].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provider: Option<Provider>,
    /// The names of the agents it is attached to, in the order they
    /// were attached; empty for a tool attached nowhere.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub agents: Vec<String>,
}
