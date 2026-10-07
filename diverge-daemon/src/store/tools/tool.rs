//! One tool, as the store holds it.

use chrono::{DateTime, Utc};
use diverge_sdk::daemon::creator::{self, Creator};
use diverge_sdk::daemon::endpoints::agents::create::client::request::{FuseMount, Provider};
use diverge_sdk::daemon::endpoints::agents::logs::server::response::{self as logs, Identity};
use diverge_sdk::daemon::endpoints::tools::Admission;
use diverge_sdk::daemon::endpoints::tools::list::server::response;
use diverge_sdk::daemon::endpoints::tools::routes::Path;
use diverge_sdk::daemon::key;

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
    /// Joined to a container somebody else runs.
    Connected {
        /// Which provider runs it.
        provider: Identity,
        /// The container's id there.
        id: String,
        /// What is offered to its runner at every join. Never
        /// reported.
        authorization: String,
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
    /// The deployer agent as it was when named, if any.
    pub deployer: Option<creator::Agent>,
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

impl Tool {
    /// The template it was made from, for a created tool.
    pub fn template(&self) -> Option<&str> {
        match &self.origin {
            Origin::Created { template, .. } => Some(template),
            Origin::Connected { .. } => None,
        }
    }

    /// Whether it is a connected tool: somebody else's, which the
    /// daemon never runs and never admits anyone to.
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
            Origin::Connected { provider, id, .. } => key::Origin::Connected {
                provider: provider.clone(),
                id: id.clone(),
            },
        };
        key::Tool {
            origin,
            index: self.index,
            name: self.name.clone(),
        }
    }

    /// The tool as a creator, or as a route names it: its template,
    /// its index and its name. A connected tool, made from no
    /// template, is neither.
    pub fn snapshot(&self) -> Option<creator::Tool> {
        self.template().map(|template| creator::Tool {
            template: template.to_string(),
            index: self.index,
            name: self.name.clone(),
        })
    }

    /// The tool as a list reports it, given whether its container
    /// runs or its connect scope is held now, the container's id
    /// while it runs, the agents it is attached to in attach order,
    /// the positions routed to it, and its admissions in the order
    /// admitted.
    pub fn report(
        &self,
        active: bool,
        running: Option<String>,
        agents: Vec<key::Agent>,
        routes: Vec<Path>,
        admissions: Vec<Admission>,
    ) -> response::Tool {
        let origin = match &self.origin {
            Origin::Created { template, .. } => response::Origin::Created {
                template: template.clone(),
                provider: self.last_provider.clone().map(|identity| logs::Provider { identity }),
                id: running,
            },
            Origin::Connected { provider, id, .. } => response::Origin::Connected {
                provider: provider.clone(),
                id: id.clone(),
            },
        };
        response::Tool {
            name: self.name.clone(),
            origin,
            index: self.index,
            creator: self.creator.clone(),
            deployer_agent: self.deployer.clone(),
            created: self.created,
            active,
            last_active: self.last_active,
            agents,
            tags: self.tags.clone(),
            routes,
            admissions,
        }
    }
}
