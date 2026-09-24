//! One agent, as the daemon holds it.

use chrono::{DateTime, Utc};
use diverge_provider_sdk::shared::containers::request::Image;
use serde::{Deserialize, Serialize};

use crate::endpoints::agents::logs::server::response::Provider;

/// One agent of the caller's: what the daemon knows of it without
/// reading its log.
///
/// Everything here the daemon holds for the agent's life, so a list
/// costs no more than the agents it names. What an agent has SAID is
/// its [`logs`](crate::endpoints::agents::logs), read separately;
/// what it was made from beyond the image — its limits, its mounts,
/// its arguments — is the create's, and is not repeated here.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Agent {
    /// The name, as its create gave it.
    pub name: String,
    /// The image it runs: the name and the digest the create named.
    pub image: Image,
    /// When the create made it.
    pub created: DateTime<Utc>,
    /// Whether the agent is active now: a loop is running in it.
    pub active: bool,
    /// When its activity last changed: the time it became active, if
    /// it is; the time it last ceased to be, if it is not; absent
    /// for an agent that has never been active. The `created` of its
    /// latest `active` or `inactive` log item.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_active: Option<DateTime<Utc>>,
    /// The provider it runs on, if active, or last ran on; absent
    /// for an agent that has never been active. See [`Provider`].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provider: Option<Provider>,
    /// The `logs_index` of its latest log item: how long its log is,
    /// and where a read that wants only what comes next starts.
    /// `0` for an empty log.
    pub logs_index: u64,
}
