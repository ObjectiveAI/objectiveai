//! Which scope is a container's: its role, from its identity.

use diverge_sdk::daemon::endpoints::postgres::{Container, Dependency as Scoped, Parent};
use diverge_sdk::daemon::reference;
use diverge_sdk::shared::containers::dependencies::Database;
use sha2::{Digest, Sha256};
use sqlx::PgConnection;

use crate::containers::{Dependency, Key, ToolKey};
use crate::daemon::Daemon;
use crate::store::agents::Agent;
use crate::store::tools::{Origin, Tool};
use crate::store::{self, agents, tools};

/// The role, and the schema, of the container's scope: `diverge_` and
/// the first forty hex digits of the SHA-256 of the container's
/// once-and-for-all identity as the wire names it — forty-eight bytes
/// of `[a-z0-9_]`, stable across restarts, never reused because the
/// identity is never reused, and safe to write as a bare identifier.
pub fn role_of(container: &Container) -> String {
    let json = serde_json::to_vec(container).unwrap_or_default();
    let digest = hex::encode(Sha256::digest(&json));
    format!("diverge_{}", &digest[..40])
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
        Origin::Connected { provider, id, .. } => reference::Tool::Connected {
            provider: provider.clone(),
            id: id.clone(),
        },
    })
}

/// The dependency's scope as the database names it: by its parent
/// agent, once and for all, or by that agent's template, as the
/// dependency's template says, and by its declared name.
pub fn container_of_dependency(dependency: &Dependency) -> Container {
    let parent = match dependency.template.database {
        Database::PerAgentInstance => Parent::Agent(reference::Agent::TemplateIndex {
            template: dependency.agent_key.template.clone(),
            index: dependency.agent_key.index,
        }),
        Database::PerAgentTemplate => Parent::AgentTemplate {
            template: dependency.agent_key.template.clone(),
        },
    };
    Container::Dependency(Scoped {
        parent,
        name: dependency.name.clone(),
    })
}

/// The container the key names, as the database names it: a record's
/// if its record is there, a dependency's if it runs.
pub async fn container_of(daemon: &Daemon, conn: &mut PgConnection, key: Key) -> Result<Option<Container>, store::Error> {
    Ok(match key {
        Key::Agent(id) => agents::by_id(conn, id, false).await?.map(|agent| container_of_agent(&agent)),
        Key::Tool(ToolKey::Record(id)) => tools::by_id(conn, id, false).await?.map(|tool| container_of_tool(&tool)),
        Key::Tool(tool @ ToolKey::Dependency(_)) => daemon
            .live
            .tool_run(tool)
            .await
            .and_then(|run| run.dependency.as_ref().map(container_of_dependency)),
    })
}
