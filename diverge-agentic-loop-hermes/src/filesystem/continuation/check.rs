//! Proving a delivered database opens, and naming the session in it.

use std::path::Path;

use sqlx::{Connection as _, Row as _};

use super::db::{open, tip};
use super::{CheckError, HERMES_HOME, STATE_DB};

/// Prove the delivered database opens, and name the session to
/// resume: `PRAGMA quick_check`, one sequential read of the file,
/// once per run — before the gateway starts — then the most
/// recently active row of `sessions`.
///
/// The check is not caution for its own sake: Hermes HEALS a
/// database it cannot open, by quarantining it and starting fresh,
/// and a run that started fresh would harvest an amnesiac
/// continuation over the lineage without anyone noticing. So a
/// delivery that does not check out fails loudly here instead
/// ([`CheckError::Corrupt`]).
///
/// The session is read here because the database is the only
/// authority on it: Hermes's compaction splits a session into a
/// child row, so the id a run started with is not necessarily the
/// lineage's tip by the time it ends. The tip is the row most
/// recently active — `last_activity_at`, or `started_at` for a row
/// that never recorded activity. A delivered database with no
/// session at all is not a continuation
/// ([`CheckError::NoSession`]).
pub async fn check() -> Result<String, CheckError> {
    let db = Path::new(HERMES_HOME).join(STATE_DB);
    let mut connection = open(&db).await?;
    let verdict = sqlx::query("PRAGMA quick_check")
        .persistent(false)
        .fetch_all(&mut connection)
        .await?
        .iter()
        .map(|row| row.get::<String, _>(0))
        .collect::<Vec<_>>();
    if verdict != ["ok"] {
        // The close's own failure is beside the point now.
        let _ = connection.close().await;
        return Err(CheckError::Corrupt(verdict));
    }
    let session = tip(&mut connection).await?;
    connection.close().await?;
    session.ok_or(CheckError::NoSession)
}
