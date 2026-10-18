//! Reading vault keys off rows.

use std::collections::HashMap;

use chrono::{DateTime, Utc};
use diverge_sdk::daemon::creator::Creator;
use serde_json::Value;
use sqlx::postgres::PgRow;
use sqlx::types::Json;
use sqlx::{PgConnection, Row as _};

use super::{Entry, Slot};
use crate::store::{Error, agents, tools};

/// The columns every vault query selects.
pub(super) const SELECT: &str = "SELECT key, tags, created, creator FROM diverge.vault";

/// The order: oldest first.
pub(super) const ORDER: &str = "ORDER BY created, key";

/// The keys the rows of [`SELECT`] hold, each with its slots, found
/// in one load of the slots of every key named, and with the records
/// binding it, found in one load of the agents and one of the tools.
pub(super) async fn entries(conn: &mut PgConnection, rows: &[PgRow]) -> Result<Vec<Entry>, Error> {
    let mut entries = rows.iter().map(entry).collect::<Result<Vec<Entry>, Error>>()?;
    if entries.is_empty() {
        return Ok(entries);
    }
    let keys: Vec<String> = entries.iter().map(|entry| entry.key.clone()).collect();
    let mut slots = slots(conn, &keys).await?;
    let agents = agents::all(conn).await?;
    let tools = tools::all(conn).await?;
    for entry in &mut entries {
        entry.slots = slots.remove(&entry.key).unwrap_or_default();
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

/// The key a row holds, with no slot and bound by nobody yet.
fn entry(row: &PgRow) -> Result<Entry, Error> {
    let Json(creator): Json<Creator> = row.try_get("creator")?;
    let created: DateTime<Utc> = row.try_get("created")?;
    Ok(Entry {
        key: row.try_get("key")?,
        slots: Vec::new(),
        agents: Vec::new(),
        standalone_tools: Vec::new(),
        tags: row.try_get("tags")?,
        created,
        creator,
    })
}

/// The slots of every one of `keys`, by key, each key's by number
/// ascending; a key with none is absent.
async fn slots(conn: &mut PgConnection, keys: &[String]) -> Result<HashMap<String, Vec<Slot>>, Error> {
    let rows = sqlx::query("SELECT key, slot, value, holder FROM diverge.vault_slots WHERE key = ANY($1) ORDER BY key, slot")
        .bind(keys)
        .fetch_all(&mut *conn)
        .await?;
    let mut slots: HashMap<String, Vec<Slot>> = HashMap::new();
    for row in &rows {
        let key: String = row.try_get("key")?;
        let slot: i32 = row.try_get("slot")?;
        let Json(value): Json<Value> = row.try_get("value")?;
        let holder: Option<Json<Creator>> = row.try_get("holder")?;
        slots.entry(key).or_default().push(Slot {
            slot: u32::try_from(slot).unwrap_or(0),
            holder: holder.map(|Json(holder)| holder),
            value,
        });
    }
    Ok(slots)
}
