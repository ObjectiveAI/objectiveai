//! The tools an agent has.

use diverge_sdk::daemon::key;
use sqlx::PgConnection;

use crate::store::agents::Agent;
use crate::store::{self, tools};

/// The tools the agent has, as keys: those attached to it directly,
/// in the order they were attached, then those attached to its
/// template, in theirs, one it has both ways once.
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
    Ok(keys)
}
