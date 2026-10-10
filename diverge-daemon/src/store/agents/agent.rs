//! One agent, as the store holds it.

use chrono::{DateTime, Utc};
use diverge_sdk::daemon::creator::{self, Creator};
use diverge_sdk::daemon::endpoints::agents::create::client::request::{FuseMount, Provider};
use diverge_sdk::daemon::endpoints::agents::list::server::response;
use diverge_sdk::daemon::endpoints::agents::logs::server::response::{self as logs, Identity};
use diverge_sdk::daemon::key;

use crate::store::{AccountId, AgentId};

/// An agent row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Agent {
    /// The row.
    pub id: AgentId,
    /// The template it was made from, by id.
    pub template: String,
    /// Its index among agents made from that template.
    pub index: u64,
    /// The name, if the agent has one.
    pub name: Option<String>,
    /// The account it runs under, if any.
    pub account: Option<AccountId>,
    /// The provider it is pinned to, with that provider's volumes
    /// mounted, if any.
    pub provider: Option<Provider>,
    /// Files of other providers' volumes, served across the daemon.
    pub fuse_file_mounts: Vec<FuseMount>,
    /// Directories of other providers' volumes, likewise.
    pub fuse_directory_mounts: Vec<FuseMount>,
    /// The provider it last ran on, if it ever ran.
    pub last_provider: Option<Identity>,
    /// When it last began or ceased running.
    pub last_active: Option<DateTime<Utc>>,
    /// Its tags, sorted bytewise.
    pub tags: Vec<String>,
    /// When it was made.
    pub created: DateTime<Utc>,
    /// Who made it.
    pub creator: Creator,
}

impl Agent {
    /// The agent as a key: its template, its index, and its name as
    /// it is called now.
    pub fn key(&self) -> key::Agent {
        key::Agent {
            template: self.template.clone(),
            index: self.index,
            name: self.name.clone(),
        }
    }

    /// The agent as a creator: the same three.
    pub fn snapshot(&self) -> creator::Agent {
        creator::Agent {
            template: self.template.clone(),
            index: self.index,
            name: self.name.clone(),
        }
    }

    /// The agent as a list reports it, given whether a loop runs in it
    /// now, the tools attached to it in attach order, and how long its
    /// log is.
    pub fn report(&self, active: bool, tools: Vec<key::Tool>, logs_index: u64) -> response::Agent {
        response::Agent {
            name: self.name.clone(),
            template: self.template.clone(),
            index: self.index,
            creator: self.creator.clone(),
            created: self.created,
            active,
            last_active: self.last_active,
            provider: self.last_provider.clone().map(|identity| logs::Provider { identity }),
            logs_index,
            tools,
            tags: self.tags.clone(),
        }
    }
}
