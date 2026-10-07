//! Which records name a volume in their mounts.

use diverge_sdk::daemon::key;
use diverge_sdk::daemon::reference;
use sqlx::PgConnection;

use crate::store::agents::Agent;
use crate::store::tools::{Origin, Tool};
use crate::store::{self, agents, tools};

/// Every volume the agent's record names: the pinned provider's
/// volume mounts, and the volume of each FUSE mount.
pub fn of_agent(agent: &Agent) -> Vec<reference::Volume> {
    let mut named = Vec::new();
    if let Some(provider) = &agent.provider {
        for mount in &provider.volume_mounts {
            named.push(reference::Volume {
                provider: provider.identity.clone(),
                name: mount.volume_name.clone(),
            });
        }
    }
    for mount in agent.fuse_file_mounts.iter().chain(&agent.fuse_directory_mounts) {
        named.push(reference::Volume {
            provider: mount.provider.clone(),
            name: mount.volume_name.clone(),
        });
    }
    named.sort_by(|a, b| (format!("{:?}", a.provider), &a.name).cmp(&(format!("{:?}", b.provider), &b.name)));
    named.dedup();
    named
}

/// Every volume the tool's record names, the same way; a connected
/// tool names none.
pub fn of_tool(tool: &Tool) -> Vec<reference::Volume> {
    let mut named = Vec::new();
    if let Origin::Created {
        provider: Some(provider), ..
    } = &tool.origin
    {
        for mount in &provider.volume_mounts {
            named.push(reference::Volume {
                provider: provider.identity.clone(),
                name: mount.volume_name.clone(),
            });
        }
    }
    for mount in tool.fuse_file_mounts.iter().chain(&tool.fuse_directory_mounts) {
        named.push(reference::Volume {
            provider: mount.provider.clone(),
            name: mount.volume_name.clone(),
        });
    }
    named.sort_by(|a, b| (format!("{:?}", a.provider), &a.name).cmp(&(format!("{:?}", b.provider), &b.name)));
    named.dedup();
    named
}

/// The agents and the tools whose records name the volume in their
/// mounts, running or not, as keys, oldest record first.
pub async fn mounters(conn: &mut PgConnection, volume: &reference::Volume) -> Result<(Vec<key::Agent>, Vec<key::Tool>), store::Error> {
    let agents = agents::all(conn)
        .await?
        .iter()
        .filter(|agent| of_agent(agent).contains(volume))
        .map(Agent::key)
        .collect();
    let tools = tools::all(conn)
        .await?
        .iter()
        .filter(|tool| of_tool(tool).contains(volume))
        .map(Tool::key)
        .collect();
    Ok((agents, tools))
}
