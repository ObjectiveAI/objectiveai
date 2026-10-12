//! Which scope is a container's: its role, from its owner and its
//! part.

use diverge_sdk::daemon::endpoints::postgres::{Container, Dependency as Scoped, Parent};
use diverge_sdk::daemon::reference;
use diverge_sdk::shared::canonical;
use diverge_sdk::shared::containers::dependencies::Scope;
use serde::Serialize;
use sha2::{Digest, Sha256};
use sqlx::PgConnection;

use crate::containers::{Dependency, Key, ToolKey};
use crate::daemon::Daemon;
use crate::store::agents::Agent;
use crate::store::services::Service;
use crate::store::tools::{Origin, Tool};
use crate::store::{self, agents, services, tools};

/// Whose a scope is: what its role names first, so that everything
/// one owner has shares a prefix. An agent owns its own scope, the
/// scopes of its transient dependencies and those of its per-agent
/// services; a tool owns its own; an agent template owns the
/// per-template scopes of the services its agents declared; and a
/// global service's scope is its template's own, held up by the agent
/// templates whose agents ever declared it and swept when the last
/// of them is deleted.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Owner {
    /// An agent, once and for all.
    Agent(reference::Agent),
    /// A tool, once and for all.
    Tool(reference::Tool),
    /// An agent template, by id.
    AgentTemplate {
        /// The template.
        template: String,
    },
    /// A global service's template, by id.
    Global {
        /// The dependency tool template.
        template: String,
    },
}

/// The owner of the container's scope.
pub fn owner_of(container: &Container) -> Owner {
    match container {
        Container::Agent(agent) => Owner::Agent(agent.clone()),
        Container::Tool(tool) => Owner::Tool(tool.clone()),
        Container::Dependency(Scoped { parent, template }) => match parent {
            Parent::Agent(agent) => Owner::Agent(agent.clone()),
            Parent::AgentTemplate { template } => Owner::AgentTemplate {
                template: template.clone(),
            },
            Parent::Global => Owner::Global {
                template: template.clone(),
            },
        },
    }
}

/// The scope's part within its owner: none for a container's own
/// scope and for a global service's, whose owner is its template
/// itself; the dependency's template id for every other dependency's.
fn part_of(container: &Container) -> Option<&str> {
    match container {
        Container::Agent(_) | Container::Tool(_) => None,
        Container::Dependency(Scoped { parent: Parent::Global, .. }) => None,
        Container::Dependency(Scoped { template, .. }) => Some(template),
    }
}

/// The first twenty hexadecimal characters of the SHA-256 of the
/// value's canonical bytes.
fn twenty<T: Serialize + ?Sized>(value: &T) -> String {
    let bytes = canonical::bytes(value).unwrap_or_default();
    let digest = hex::encode(Sha256::digest(&bytes));
    digest[..20].to_string()
}

/// The role, and the schema, of the container's scope: `diverge_`,
/// the first twenty hex digits of the SHA-256 of the owner's
/// canonical bytes, then the first twenty of the SHA-256 of the part's
/// — `null` for a container's own scope and a global service's, the
/// dependency's template id as a JSON string for any other
/// dependency's. Forty-eight bytes of `[a-z0-9_]`, stable across
/// restarts and providers, never reused because the identity is never
/// reused, safe to write as a bare identifier, and OWNER FIRST: every
/// scope an owner has is under [`prefix_of`] its owner, which is how
/// a delete finds them all.
pub fn role_of(container: &Container) -> String {
    format!("diverge_{}{}", twenty(&owner_of(container)), twenty(&part_of(container)))
}

/// What every role the owner has begins with: `diverge_` and the
/// owner's twenty.
pub fn prefix_of(owner: &Owner) -> String {
    format!("diverge_{}", twenty(owner))
}

/// The key the scope's provisioning is serialized under: the first
/// eight bytes of the SHA-256 of the role, as a signed integer.
pub fn lock_key(role: &str) -> i64 {
    let digest = Sha256::digest(role.as_bytes());
    i64::from_be_bytes(digest[..8].try_into().unwrap_or([0; 8]))
}

/// The agent as the database names it.
pub fn container_of_agent(agent: &Agent) -> Container {
    Container::Agent(reference::Agent::TemplateIndex {
        template: agent.template.clone(),
        index: agent.index,
    })
}

/// The tool as the database names it.
pub fn container_of_tool(tool: &Tool) -> Container {
    Container::Tool(match &tool.origin {
        Origin::Created { template, .. } => reference::Tool::TemplateIndex {
            template: template.clone(),
            index: tool.index,
        },
        Origin::Connected { daemon, tool } => reference::Tool::Connected {
            daemon: daemon.clone(),
            tool: Box::new(tool.clone()),
        },
    })
}

/// The transient dependency's scope as the database names it: by its
/// parent agent, once and for all, which a transient tool's always
/// is, and by its template's id.
pub fn container_of_dependency(dependency: &Dependency) -> Container {
    Container::Dependency(Scoped {
        parent: Parent::Agent(reference::Agent::TemplateIndex {
            template: dependency.agent_key.template.clone(),
            index: dependency.agent_key.index,
        }),
        template: dependency.template.clone(),
    })
}

/// The service's scope as the database names it: by its one agent —
/// `owner`, loaded by the caller — its one agent template, or the
/// template alone, as its scope says, and by its template's id.
/// `None` for a per-agent service whose agent is gone.
pub fn container_of_service(service: &Service, owner: Option<&Agent>) -> Option<Container> {
    let parent = match service.scope {
        Scope::PerAgentInstance => {
            let owner = owner?;
            Parent::Agent(reference::Agent::TemplateIndex {
                template: owner.template.clone(),
                index: owner.index,
            })
        }
        Scope::PerAgentTemplate => Parent::AgentTemplate {
            template: service.owner_template.clone().unwrap_or_default(),
        },
        Scope::Global => Parent::Global,
    };
    Some(Container::Dependency(Scoped {
        parent,
        template: service.template.clone(),
    }))
}

/// The container the key names, as the database names it: a record's
/// if its record is there, a transient dependency's if it runs, a
/// service's if its row is there.
pub async fn container_of(daemon: &Daemon, conn: &mut PgConnection, key: Key) -> Result<Option<Container>, store::Error> {
    Ok(match key {
        Key::Agent(id) => agents::by_id(conn, id, false).await?.map(|agent| container_of_agent(&agent)),
        Key::Tool(ToolKey::Record(id)) => tools::by_id(conn, id, false).await?.map(|tool| container_of_tool(&tool)),
        Key::Tool(tool @ ToolKey::Dependency(_)) => daemon
            .live
            .tool_run(tool)
            .await
            .and_then(|run| run.dependency.as_ref().map(container_of_dependency)),
        Key::Tool(ToolKey::Service(id)) => {
            let Some(service) = services::by_id(conn, id, false).await? else {
                return Ok(None);
            };
            let owner = match service.owner_agent {
                Some(id) => agents::by_id(conn, id, false).await?,
                None => None,
            };
            container_of_service(&service, owner.as_ref())
        }
    })
}
