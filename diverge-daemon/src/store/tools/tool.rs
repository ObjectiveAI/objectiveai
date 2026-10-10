//! One tool, as the store holds it.

use chrono::{DateTime, Utc};
use diverge_sdk::daemon::creator::{self, Creator};
use diverge_sdk::daemon::endpoints::agents::create::client::request::{FuseMount, Provider};
use diverge_sdk::daemon::endpoints::agents::logs::server::response::{self as logs, Identity};
use diverge_sdk::daemon::endpoints::tools::list::server::response;
use diverge_sdk::daemon::key;
use diverge_sdk::daemon::reference;

use crate::store::{AccountId, ToolId};

/// How a tool came to be, and what that way holds.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Origin {
    /// Made from a template, to run on the provider it is pinned to
    /// with that provider's volumes, or on one the daemon chooses.
    Created {
        /// The template, by id.
        template: String,
        /// The provider pin and its volumes, if any.
        provider: Option<Provider>,
    },
    /// Joined to a tool another daemon holds.
    Connected {
        /// The daemon the tool is on, by the name of its record.
        daemon: String,
        /// The tool, as that daemon names it.
        tool: reference::Tool,
    },
}

/// A tool row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Tool {
    /// The row.
    pub id: ToolId,
    /// How it came to be.
    pub origin: Origin,
    /// Its index among tools made the same way.
    pub index: u64,
    /// The name, if the tool has one.
    pub name: Option<String>,
    /// The account it runs under, if any.
    pub account: Option<AccountId>,
    /// Files of other providers' volumes, served across the daemon.
    pub fuse_file_mounts: Vec<FuseMount>,
    /// Directories of other providers' volumes, likewise.
    pub fuse_directory_mounts: Vec<FuseMount>,
    /// The provider it last ran on, or was last connected through, if it
    /// ever was.
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

impl Tool {
    /// The template it was made from, for a created tool.
    pub fn template(&self) -> Option<&str> {
        match &self.origin {
            Origin::Created { template, .. } => Some(template),
            Origin::Connected { .. } => None,
        }
    }

    /// Whether it is a connected tool: another daemon's, which this
    /// daemon never runs and never serves to another daemon.
    pub fn is_connected(&self) -> bool {
        matches!(self.origin, Origin::Connected { .. })
    }

    /// The tool as a key: its origin, its index, and its name as it
    /// is called now.
    pub fn key(&self) -> key::Tool {
        let origin = match &self.origin {
            Origin::Created { template, .. } => key::Origin::Created {
                template: template.clone(),
            },
            Origin::Connected { daemon, tool } => key::Origin::Connected {
                daemon: daemon.clone(),
                tool: Box::new(tool.clone()),
            },
        };
        key::Tool::Record {
            origin,
            index: self.index,
            name: self.name.clone(),
        }
    }

    /// The tool as a creator: its template, its index and its name.
    /// A connected tool, made from no template, is none.
    pub fn snapshot(&self) -> Option<creator::Tool> {
        self.template().map(|template| creator::Tool::Record {
            template: template.to_string(),
            index: self.index,
            name: self.name.clone(),
        })
    }

    /// The tool as a list reports it, given whether its container
    /// runs or its connect scope is held now, the container's id
    /// while it runs, and the agents it is attached to in attach
    /// order.
    pub fn report(&self, active: bool, running: Option<String>, agents: Vec<key::Agent>) -> response::Tool {
        let origin = match &self.origin {
            Origin::Created { template, .. } => response::Origin::Created {
                template: template.clone(),
                index: self.index,
                provider: self.last_provider.clone().map(|identity| logs::Provider { identity }),
                id: running,
            },
            Origin::Connected { daemon, tool } => response::Origin::Connected {
                daemon: daemon.clone(),
                tool: tool.clone(),
                index: self.index,
            },
        };
        response::Tool {
            name: self.name.clone(),
            origin,
            creator: self.creator.clone(),
            created: self.created,
            active,
            last_active: self.last_active,
            agents,
            tags: self.tags.clone(),
        }
    }
}
