//! One dependency tool, as the store holds it.

use chrono::{DateTime, Utc};
use diverge_sdk::daemon::creator::Creator;
use diverge_sdk::daemon::endpoints::dependency_tools::list::server::response::{Attached, Listed, Run};
use diverge_sdk::daemon::key;
use diverge_sdk::shared::containers::dependencies::{Mode, Scope, Template};

use crate::store::agents::Agent;
use crate::store::{AgentId, DependencyToolId};

/// A dependency tool row.
#[derive(Debug, Clone, PartialEq)]
pub struct DependencyTool {
    /// The row.
    pub id: DependencyToolId,
    /// The template it is of, by id.
    pub template: String,
    /// The template, whole.
    pub declared: Template,
    /// How far it is shared: what its scope key is made of. A
    /// transient template's is `instance`.
    pub scope: Scope,
    /// The one agent it is for, when its scope is per agent.
    pub owner_agent: Option<AgentId>,
    /// The one agent template it is for, when its scope is per agent
    /// template.
    pub owner_template: Option<String>,
    /// Its tags, sorted bytewise.
    pub tags: Vec<String>,
    /// When it was made.
    pub created: DateTime<Utc>,
    /// The agent whose registration made it.
    pub creator: Creator,
}

impl DependencyTool {
    /// Whether it is a service, kept by the daemon, rather than a
    /// transient tool of its agent's run.
    pub fn is_service(&self) -> bool {
        matches!(self.declared.mode, Mode::Service { .. })
    }

    /// Whose it is, as the wire names it: the owner agent, handed in
    /// by the caller for a per-agent tool since a name is as it is
    /// called now, its agent template, or nobody. `None` for a
    /// per-agent tool whose owner the caller did not hand in.
    pub fn owner(&self, owner_agent: Option<&Agent>) -> Option<key::Owner> {
        Some(match self.scope {
            Scope::Instance => key::Owner::Agent(owner_agent?.key()),
            Scope::Template => key::Owner::AgentTemplate {
                template: self.owner_template.clone().unwrap_or_default(),
            },
            Scope::Global => key::Owner::Global,
        })
    }

    /// The tool as a key: its template and whose it is.
    pub fn key(&self, owner_agent: Option<&Agent>) -> Option<key::Tool> {
        Some(key::Tool::Dependency {
            template: self.template.clone(),
            owner: self.owner(owner_agent)?,
        })
    }

    /// The tool as a list reports it, with its owner, the agents
    /// attached to it and its run, loaded by the caller.
    pub fn report(&self, owner: key::Owner, agents: Vec<Attached>, run: Option<Run>) -> Listed {
        Listed {
            id: self.template.clone(),
            owner,
            created: self.created,
            creator: self.creator.clone(),
            tags: self.tags.clone(),
            template: self.declared.clone(),
            agents,
            run,
        }
    }
}

/// The scope's word, as the `scope` column holds it: the snake case
/// of the wire.
pub fn scope_name(scope: Scope) -> &'static str {
    match scope {
        Scope::Instance => "instance",
        Scope::Template => "template",
        Scope::Global => "global",
    }
}

/// The scope a column's word names, or `None` for a word that is
/// none of the three.
pub fn scope_of(name: &str) -> Option<Scope> {
    match name {
        "instance" => Some(Scope::Instance),
        "template" => Some(Scope::Template),
        "global" => Some(Scope::Global),
        _ => None,
    }
}
