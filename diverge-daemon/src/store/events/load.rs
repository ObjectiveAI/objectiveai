//! Loading events.

use diverge_sdk::daemon::creator::Creator;
use sqlx::types::Json;
use sqlx::{PgConnection, Row as _};

use super::{Event, action_of};
use crate::store::Error;

/// Every event, by id ascending: the order the edges were added in.
pub async fn all(conn: &mut PgConnection) -> Result<Vec<Event>, Error> {
    let rows = sqlx::query("SELECT id, action, sender, receiver, count FROM diverge.events ORDER BY id")
        .fetch_all(&mut *conn)
        .await?;
    let mut events = Vec::with_capacity(rows.len());
    for row in &rows {
        let id: i64 = row.try_get("id")?;
        let name: String = row.try_get("action")?;
        let Some(action) = action_of(&name) else {
            return Err(Error::Json(serde::de::Error::custom(format!("`{name}` is not an event action"))));
        };
        let Json(sender): Json<Creator> = row.try_get("sender")?;
        let Json(receiver): Json<Creator> = row.try_get("receiver")?;
        let count: i64 = row.try_get("count")?;
        events.push(Event {
            id: u64::try_from(id).unwrap_or(0),
            sender,
            receiver,
            action,
            count: u64::try_from(count).unwrap_or(0),
        });
    }
    Ok(events)
}
