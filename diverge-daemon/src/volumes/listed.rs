//! One volume as a list reports it.

use chrono::{DateTime, Utc};
use diverge_sdk::daemon::endpoints::agents::logs::server::response::Identity;
use diverge_sdk::daemon::endpoints::volumes::list::server::response;
use diverge_sdk::daemon::key;
use diverge_sdk::daemon::reference;
use diverge_sdk::provider::endpoints::volumes::list::server::response::Volume;

/// A volume: the provider that holds it, what the provider's listing
/// says of it, and the agents and tools of the daemon's whose records
/// name it in their mounts.
#[derive(Debug, Clone, PartialEq)]
pub struct Listed {
    /// The provider.
    pub provider: Identity,
    /// The listing's entry.
    pub volume: Volume,
    /// The agents that mount it, running or not.
    pub agents: Vec<key::Agent>,
    /// The tools that mount it, running or not.
    pub tools: Vec<key::Tool>,
}

impl Listed {
    /// How the wire names it.
    pub fn reference(&self) -> reference::Volume {
        reference::Volume {
            provider: self.provider.clone(),
            name: self.volume.name.clone(),
        }
    }

    /// Whether some agent or tool names it in its mounts.
    pub fn mounted(&self) -> bool {
        !self.agents.is_empty() || !self.tools.is_empty()
    }

    /// When the provider says it came into being.
    pub fn created(&self) -> DateTime<Utc> {
        DateTime::<Utc>::from_timestamp(i64::try_from(self.volume.created).unwrap_or(0), 0).unwrap_or_default()
    }

    /// The volume as a list reports it.
    pub fn report(&self) -> response::Volume {
        response::Volume {
            provider: self.provider.clone(),
            name: self.volume.name.clone(),
            bytes: self.volume.bytes,
            mode: self.volume.mode,
            created: self.created(),
            agents: self.agents.clone(),
            tools: self.tools.clone(),
        }
    }
}
