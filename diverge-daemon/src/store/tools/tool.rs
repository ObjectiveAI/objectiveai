//! One tool, as the store holds it.

use chrono::{DateTime, Utc};
use diverge_sdk::daemon::creator::{self, Creator};
use diverge_sdk::daemon::endpoints::agents::create::client::request::{FuseMount, Provider};
use diverge_sdk::daemon::agent_log::{self as logs, Identity};
use diverge_sdk::daemon::endpoints::standalone_tools::list::server::response;
use diverge_sdk::daemon::endpoints::standalone_tools::templates::Mode;
use diverge_sdk::daemon::key;
use indexmap::IndexMap;

use crate::store::ToolId;
use diverge_sdk::shared::permission::Grant;

/// A tool row: made from a template, to run on the provider it is
/// pinned to with that provider's volumes, or on one the daemon
/// chooses.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Tool {
    /// The row.
    pub id: ToolId,
    /// The template it was made from, by id.
    pub template: String,
    /// The provider pin and its volumes, if any.
    pub provider: Option<Provider>,
    /// Its index among tools made from its template.
    pub index: u64,
    /// The name, if the tool has one.
    pub name: Option<String>,
    /// Files of other providers' volumes, served across the daemon.
    pub fuse_file_mounts: Vec<FuseMount>,
    /// Directories of other providers' volumes, likewise.
    pub fuse_directory_mounts: Vec<FuseMount>,
    /// Which vault key each secret its template declares is.
    pub secrets: IndexMap<String, String>,
    /// What its container declared it may ask of the daemon when it
    /// last started; empty for one that declared none or has never
    /// started.
    pub permissions: Vec<Grant>,
    /// The provider it last ran on, if it ever did.
    pub last_provider: Option<Identity>,
    /// When its container last came up or went down.
    pub last_deployed: Option<DateTime<Utc>>,
    /// When its calls and reads in flight last began or ceased.
    pub last_active: Option<DateTime<Utc>>,
    /// Its tags, sorted bytewise.
    pub tags: Vec<String>,
    /// When it was made.
    pub created: DateTime<Utc>,
    /// Who made it.
    pub creator: Creator,
}

impl Tool {
    /// The tool as a key: its template, its index, and its name as it
    /// is called now.
    pub fn key(&self) -> key::Tool {
        key::Tool::Record {
            template: self.template.clone(),
            index: self.index,
            name: self.name.clone(),
        }
    }

    /// The tool as a creator: its template, its index and its name.
    pub fn snapshot(&self) -> creator::Tool {
        creator::Tool::Record {
            template: self.template.clone(),
            index: self.index,
            name: self.name.clone(),
        }
    }

    /// The record as a list reports it, given whether it is active now
    /// (a call or a read in flight), whether it is deployed, its
    /// container's id while it runs, the agents and the agent
    /// templates it is attached to, and its template's mode — the
    /// default for a template that is gone.
    pub fn report(&self, active: bool, deployed: bool, running: Option<String>, agents: Vec<key::Agent>, templates: Vec<String>, mode: Option<Mode>) -> response::Tool {
        response::Tool {
            name: self.name.clone(),
            template: self.template.clone(),
            index: self.index,
            mode: mode.unwrap_or_default(),
            provider: self.last_provider.clone().map(|identity| logs::Provider { identity }),
            id: running,
            creator: self.creator.clone(),
            created: self.created,
            active,
            last_active: self.last_active,
            deployed,
            last_deployed: self.last_deployed,
            attached_agent_instances: agents,
            attached_agent_templates: templates,
            permissions: self.permissions.clone(),
            tags: self.tags.clone(),
        }
    }
}
