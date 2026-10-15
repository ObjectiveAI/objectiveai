//! Which scope is a container's: its role, from its owner and its
//! part.

use diverge_sdk::daemon::endpoints::postgres::Container;
use diverge_sdk::daemon::reference;
use diverge_sdk::shared::canonical;
use diverge_sdk::shared::containers::dependencies::Scope;
use serde::Serialize;
use sha2::{Digest, Sha256};
use sqlx::PgConnection;

use crate::containers::{Key, ToolKey};
use crate::store::agents::Agent;
use crate::store::dependency_tools::DependencyTool;
use crate::store::tools::{Origin, Tool};
use crate::store::{self, agents, dependency_tools, tools};

/// Whose a scope is: what its role names first. An agent owns its own
/// scope and those of its instance dependency tools; a tool owns its
/// own; an agent template owns the scopes of its per-template
/// dependency tools; and a global dependency tool's scope is its
/// template's own. A dependency tool's scope goes with its row: an
/// instance tool's at its agent's delete, a per-template tool's at its
/// agent template's, every tool's at the template's; an agent's own
/// scope goes with nothing.
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
        /// The dependency template.
        template: String,
    },
}

/// The owner of the container's scope.
pub fn owner_of(container: &Container) -> Owner {
    match container {
        Container::Agent(agent) => Owner::Agent(agent.clone()),
        Container::Tool(tool) => Owner::Tool(tool.clone()),
        Container::Dependency(reference::Dependency { owner, template }) => match owner {
            reference::Owner::Agent(agent) => Owner::Agent(agent.clone()),
            reference::Owner::AgentTemplate { template } => Owner::AgentTemplate {
                template: template.clone(),
            },
            reference::Owner::Global => Owner::Global {
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
        Container::Dependency(reference::Dependency {
            owner: reference::Owner::Global,
            ..
        }) => None,
        Container::Dependency(reference::Dependency { template, .. }) => Some(template),
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
/// reused, and safe to write as a bare identifier, and computable from
/// the row at any time, which is how a delete finds the scope to drop.
pub fn role_of(container: &Container) -> String {
    format!("diverge_{}{}", twenty(&owner_of(container)), twenty(&part_of(container)))
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
        Origin::Connected { daemon, account, tool } => reference::Tool::Connected {
            daemon: daemon.clone(),
            account: account.clone(),
            tool: Box::new(tool.clone()),
        },
    })
}

/// The dependency tool's scope as the database names it: by its
/// template's id and its one agent — `owner`, loaded by the caller —
/// its one agent template, or everyone, as its scope says. `None` for
/// a per-agent tool whose agent the caller did not hand in.
pub fn container_of_dependency_tool(tool: &DependencyTool, owner: Option<&Agent>) -> Option<Container> {
    let owner = match tool.scope {
        Scope::Instance => {
            let owner = owner?;
            reference::Owner::Agent(reference::Agent::TemplateIndex {
                template: owner.template.clone(),
                index: owner.index,
            })
        }
        Scope::Template => reference::Owner::AgentTemplate {
            template: tool.owner_template.clone().unwrap_or_default(),
        },
        Scope::Global => reference::Owner::Global,
    };
    Some(Container::Dependency(reference::Dependency {
        template: tool.template.clone(),
        owner,
    }))
}

/// The container the key names, as the database names it: a record's
/// if its record is there, a dependency tool's if its row is.
pub async fn container_of(conn: &mut PgConnection, key: Key) -> Result<Option<Container>, store::Error> {
    Ok(match key {
        Key::Agent(id) => agents::by_id(conn, id, false).await?.map(|agent| container_of_agent(&agent)),
        Key::Tool(ToolKey::Record(id)) => tools::by_id(conn, id, false).await?.map(|tool| container_of_tool(&tool)),
        Key::Tool(ToolKey::Dependency(id)) => {
            let Some(tool) = dependency_tools::by_id(conn, id, false).await? else {
                return Ok(None);
            };
            let owner = match tool.owner_agent {
                Some(id) => agents::by_id(conn, id, false).await?,
                None => None,
            };
            container_of_dependency_tool(&tool, owner.as_ref())
        }
    })
}
