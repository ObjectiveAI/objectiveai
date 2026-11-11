//! Loading events.

use sqlx::{PgConnection, Row as _};

use super::Event;
use super::party::party;
use crate::store::Error;

/// Every event, by id ascending: the order the edges were added in,
/// each end joined to the row it names for that row's template and
/// index — at most one of an end's three joins finds a row, the one
/// its kind uses, and none does for an account.
pub async fn all(conn: &mut PgConnection) -> Result<Vec<Event>, Error> {
    let rows = sqlx::query(
        "SELECT e.id, e.count, \
         e.sender_kind, e.sender_issuer, e.sender_subject, \
         COALESCE(sa.template, st.template, sd.template) AS sender_template, \
         COALESCE(sa.index, st.index, sd.index) AS sender_index, \
         e.receiver_kind, e.receiver_issuer, e.receiver_subject, \
         COALESCE(ra.template, rt.template, rd.template) AS receiver_template, \
         COALESCE(ra.index, rt.index, rd.index) AS receiver_index \
         FROM diverge.events e \
         LEFT JOIN diverge.agents sa ON sa.id = e.sender_agent \
         LEFT JOIN diverge.tools st ON st.id = e.sender_tool \
         LEFT JOIN diverge.dependency_tools sd ON sd.id = e.sender_dependency \
         LEFT JOIN diverge.agents ra ON ra.id = e.receiver_agent \
         LEFT JOIN diverge.tools rt ON rt.id = e.receiver_tool \
         LEFT JOIN diverge.dependency_tools rd ON rd.id = e.receiver_dependency \
         ORDER BY e.id",
    )
    .fetch_all(&mut *conn)
    .await?;
    let mut events = Vec::with_capacity(rows.len());
    for row in &rows {
        let id: i64 = row.try_get("id")?;
        let count: i64 = row.try_get("count")?;
        events.push(Event {
            id: u64::try_from(id).unwrap_or(0),
            sender: party(row, "sender")?,
            receiver: party(row, "receiver")?,
            count: u64::try_from(count).unwrap_or(0),
        });
    }
    Ok(events)
}
