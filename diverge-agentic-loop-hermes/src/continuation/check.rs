//! Proving a delivered database opens.

use std::path::Path;

use sqlx::{Connection as _, Row as _};

use super::db::open;
use super::{CheckError, HERMES_HOME, STATE_DB};

/// Prove the delivered database opens: `PRAGMA quick_check`, one
/// sequential read of the file, once per run — before the gateway
/// starts.
///
/// Not caution for its own sake: Hermes HEALS a database it cannot
/// open, by quarantining it and starting fresh, and a run that
/// started fresh would harvest an amnesiac continuation over the
/// lineage without anyone noticing. So a delivery that does not
/// check out fails loudly here instead ([`CheckError::Corrupt`]).
pub async fn check() -> Result<(), CheckError> {
    let db = Path::new(HERMES_HOME).join(STATE_DB);
    let mut connection = open(&db).await?;
    let verdict = sqlx::query("PRAGMA quick_check")
        .persistent(false)
        .fetch_all(&mut connection)
        .await?
        .iter()
        .map(|row| row.get::<String, _>(0))
        .collect::<Vec<_>>();
    connection.close().await?;
    if verdict != ["ok"] {
        return Err(CheckError::Corrupt(verdict));
    }
    Ok(())
}
