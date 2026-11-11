//! The tools an agent has.

use diverge_sdk::daemon::key;
use sqlx::PgConnection;

use crate::store::agents::Agent;
use crate::store::{self, connected_standalone_tools, tools};

/// The tools the agent has, as keys: its standalone tools — those
/// attached to it directly, in the order they were attached, then those
/// attached to its template, in theirs, one it has both ways once —
/// then its connected tools the same way.
pub async fn tools_of(conn: &mut PgConnection, agent: &Agent) -> Result<Vec<key::Tool>, store::Error> {
    let mut ids = tools::attachments::of_agent(conn, agent.id).await?;
    for id in tools::template_attachments::of_template(conn, &agent.template).await? {
        if !ids.contains(&id) {
            ids.push(id);
        }
    }
    let mut keys = Vec::new();
    for id in ids {
        if let Some(tool) = tools::by_id(conn, id, false).await? {
            keys.push(tool.key());
        }
    }
    let mut ids = connected_standalone_tools::attachments::of_agent(conn, agent.id).await?;
    for id in connected_standalone_tools::template_attachments::of_template(conn, &agent.template).await? {
        if !ids.contains(&id) {
            ids.push(id);
        }
    }
    for id in ids {
        if let Some(tool) = connected_standalone_tools::by_id(conn, id, false).await? {
            keys.push(tool.key());
        }
    }
    Ok(keys)
}
