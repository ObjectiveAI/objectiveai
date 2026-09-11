//! The session to resume, read from the landed database.

use sqlx::Connection as _;

use super::continuation::db::{open, tip};
use super::{HERMES_HOME, STATE_DB};

/// The lineage's tip — the most recently active session in
/// `state.db` — or `None` for a database with no session yet.
///
/// The same reading [`continuation::check`](super::continuation::check)
/// makes after a restore, repeated between turns and at every later
/// run's start: Hermes's
/// compaction can rotate a session into a child row mid-run, so the
/// id a turn was started with is not necessarily the one the next
/// turn should record into. The database is the only authority.
pub async fn session() -> Result<Option<String>, sqlx::Error> {
    let db = std::path::Path::new(HERMES_HOME).join(STATE_DB);
    // No database yet — a fresh lineage whose first run has not
    // started a gateway — is no session, not an error.
    if !db.exists() {
        return Ok(None);
    }
    let mut connection = open(&db).await?;
    let session = tip(&mut connection).await?;
    connection.close().await?;
    Ok(session)
}
