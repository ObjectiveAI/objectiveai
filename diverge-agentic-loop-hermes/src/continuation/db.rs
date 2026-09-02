//! The SQLite side: one connection, and the fold that leaves a lone
//! file.

use std::path::Path;

use sqlx::sqlite::{SqliteConnectOptions, SqliteConnection};
use sqlx::{ConnectOptions as _, Connection as _, Row as _};

use super::ReadError;

/// Fold the write-ahead log into `state.db`: see
/// [`stream`](super::stream).
pub(super) async fn fold(db: &Path) -> Result<(), ReadError> {
    let mut connection = open(db).await?;
    // Autocommit: nothing here opened a transaction, and VACUUM
    // refuses to run inside one. Not persistent: a cached prepared
    // statement would outlive this call in the connection's cache.
    sqlx::query("VACUUM")
        .persistent(false)
        .execute(&mut connection)
        .await?;
    let row = sqlx::query("PRAGMA wal_checkpoint(TRUNCATE)")
        .persistent(false)
        .fetch_one(&mut connection)
        .await?;
    // Column 0 is 1 when the checkpoint was blocked from completing.
    let blocked = row.get::<i64, _>(0);
    if blocked != 0 {
        // The close's own failure is beside the point now.
        let _ = connection.close().await;
        return Err(ReadError::Busy);
    }
    connection.close().await?;
    Ok(())
}

/// One connection to the database, read/write, never creating: a
/// missing file is an error, not a fresh start. Never a pool — a
/// pool's idle connections would keep the last-close cleanup from
/// ever firing. Nothing else is set: no journal mode (the file's
/// own stays), no optimize-on-close (that would write through the
/// log after the fold).
pub(super) async fn open(db: &Path) -> Result<SqliteConnection, sqlx::Error> {
    SqliteConnectOptions::new()
        .filename(db)
        .create_if_missing(false)
        .read_only(false)
        .connect()
        .await
}
