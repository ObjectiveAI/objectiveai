//! One route, as the store holds it.

use chrono::{DateTime, Utc};
use diverge_sdk::daemon::creator::{self, Creator};
use diverge_sdk::daemon::endpoints::tools::routes::Path;
use diverge_sdk::daemon::endpoints::tools::routes::list::server::response;

use crate::store::ToolId;

/// A route row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Route {
    /// The agent that asks, by name.
    pub agent: String,
    /// The dependency's template.
    pub template: String,
    /// The tool that answers there.
    pub tool: ToolId,
    /// When it was put down.
    pub created: DateTime<Utc>,
    /// Who put it down.
    pub creator: Creator,
}

impl Route {
    /// The position.
    pub fn path(&self) -> Path {
        Path {
            agent: self.agent.clone(),
            template: self.template.clone(),
        }
    }

    /// The route as a list reports it, given the tool as it is called
    /// now.
    pub fn report(&self, tool: creator::Tool) -> response::Route {
        response::Route {
            path: self.path(),
            tool,
            created: self.created,
            creator: self.creator.clone(),
        }
    }
}
