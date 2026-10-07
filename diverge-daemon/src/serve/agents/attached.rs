//! The tools attached to an agent.

use diverge_sdk::daemon::key;
use sqlx::PgConnection;

use crate::store::{self, AgentId, tools};

/// The tools attached to the agent, as keys, in the order they were
/// attached.
pub async fn tools_of(conn: &mut PgConnection, agent: AgentId) -> Result<Vec<key::Tool>, store::Error> {
    let mut keys = Vec::new();
    for id in tools::attachments::of_agent(conn, agent).await? {
        if let Some(tool) = tools::by_id(conn, id, false).await? {
            keys.push(tool.key());
        }
    }
    Ok(keys)
}
