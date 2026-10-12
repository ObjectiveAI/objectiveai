//! One service, as the store holds it.

use chrono::{DateTime, Utc};
use diverge_sdk::daemon::creator::Creator;
use diverge_sdk::daemon::endpoints::agents::logs::server::response::Identity;
use diverge_sdk::daemon::key;
use diverge_sdk::shared::containers::dependencies::{Scope, Template};

use crate::store::agents::Agent;
use crate::store::{AgentId, ServiceId};

/// A service row.
#[derive(Debug, Clone, PartialEq)]
pub struct Service {
    /// The row.
    pub id: ServiceId,
    /// The dependency tool template it was made from, by id.
    pub template: String,
    /// How far it is shared: what its scope key is made of.
    pub scope: Scope,
    /// The one agent it is for, when its scope is per agent.
    pub owner_agent: Option<AgentId>,
    /// The one agent template it is for, when its scope is per agent
    /// template.
    pub owner_template: Option<String>,
    /// The template it was made from, whole.
    pub declared: Template,
    /// The provider it last ran on, if it ever ran.
    pub last_provider: Option<Identity>,
    /// When it was made.
    pub created: DateTime<Utc>,
    /// The agent whose registration made it.
    pub creator: Creator,
}

impl Service {
    /// Whose it is, as the wire names it: the owner agent, handed in
    /// by the caller for a per-agent service since a name is as it is
    /// called now, its template, or nobody. `None` for a per-agent
    /// service whose owner the caller did not hand in.
    pub fn owner(&self, owner_agent: Option<&Agent>) -> Option<key::Owner> {
        Some(match self.scope {
            Scope::PerAgentInstance => key::Owner::Agent(owner_agent?.key()),
            Scope::PerAgentTemplate => key::Owner::AgentTemplate {
                template: self.owner_template.clone().unwrap_or_default(),
            },
            Scope::Global => key::Owner::Global,
        })
    }

    /// The service as a key: its template and whose it is.
    pub fn key(&self, owner_agent: Option<&Agent>) -> Option<key::Tool> {
        Some(key::Tool::Service {
            template: self.template.clone(),
            owner: self.owner(owner_agent)?,
        })
    }
}

/// The scope's word, as the `scope` column holds it: the snake case
/// of the wire.
pub fn scope_name(scope: Scope) -> &'static str {
    match scope {
        Scope::PerAgentInstance => "per_agent_instance",
        Scope::PerAgentTemplate => "per_agent_template",
        Scope::Global => "global",
    }
}

/// The scope a column's word names, or `None` for a word that is
/// none of the three.
pub fn scope_of(name: &str) -> Option<Scope> {
    match name {
        "per_agent_instance" => Some(Scope::PerAgentInstance),
        "per_agent_template" => Some(Scope::PerAgentTemplate),
        "global" => Some(Scope::Global),
        _ => None,
    }
}
