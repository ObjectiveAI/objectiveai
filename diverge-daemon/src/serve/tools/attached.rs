//! The agents a tool is attached to.

use diverge_sdk::daemon::key;
use sqlx::PgConnection;

use crate::store::{self, ToolId, agents, tools};

/// The agents the tool is attached to, as keys, in the order it was
/// attached to them.
pub async fn agents_of(conn: &mut PgConnection, tool: ToolId) -> Result<Vec<key::Agent>, store::Error> {
    let mut keys = Vec::new();
    for id in tools::attachments::of_tool(conn, tool).await? {
        if let Some(agent) = agents::by_id(conn, id, false).await? {
            keys.push(agent.key());
        }
    }
    Ok(keys)
}
