//! Reading vault keys off rows.

use chrono::{DateTime, Utc};
use diverge_sdk::daemon::creator::Creator;
use serde_json::Value;
use sqlx::postgres::PgRow;
use sqlx::types::Json;
use sqlx::{PgConnection, Row as _};

use super::Entry;
use crate::store::{Error, agents, tools};

/// The columns every vault query selects.
pub(super) const SELECT: &str = "SELECT key, value, tags, created, creator FROM diverge.vault";

/// The order: oldest first.
pub(super) const ORDER: &str = "ORDER BY created, key";

/// The keys the rows of [`SELECT`] hold, each with the records
/// binding it, found in one load of the agents and one of the tools.
pub(super) async fn entries(conn: &mut PgConnection, rows: &[PgRow]) -> Result<Vec<Entry>, Error> {
    let mut entries = rows.iter().map(entry).collect::<Result<Vec<Entry>, Error>>()?;
    if entries.is_empty() {
        return Ok(entries);
    }
    let agents = agents::all(conn).await?;
    let tools = tools::all(conn).await?;
    for entry in &mut entries {
        entry.agents = agents
            .iter()
            .filter(|agent| agent.secrets.values().any(|key| *key == entry.key))
            .map(|agent| agent.key())
            .collect();
        entry.standalone_tools = tools
            .iter()
            .filter(|tool| tool.secrets.values().any(|key| *key == entry.key))
            .map(|tool| tool.key())
            .collect();
    }
    Ok(entries)
}

/// The key a row holds, bound by nobody yet.
fn entry(row: &PgRow) -> Result<Entry, Error> {
    let Json(value): Json<Value> = row.try_get("value")?;
    let Json(creator): Json<Creator> = row.try_get("creator")?;
    let created: DateTime<Utc> = row.try_get("created")?;
    Ok(Entry {
        key: row.try_get("key")?,
        value,
        agents: Vec::new(),
        standalone_tools: Vec::new(),
        tags: row.try_get("tags")?,
        created,
        creator,
    })
}
