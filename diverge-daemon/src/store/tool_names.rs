//! The one namespace of tool names: a name is unique across the
//! standalone tools and the connected standalone tools together.
//!
//! Each table's `name` is `UNIQUE` on its own, which no constraint can
//! carry across the two, so the namespace is kept by these rules:
//!
//! 1. **Every naming locks first.** A create that names its tool and an
//!    edit that renames one, of either family, takes [`lock`] in its
//!    transaction before it looks.
//! 2. **Then looks in both.** [`taken`] answers whether any standalone
//!    or connected tool holds the name; one that does is the request's
//!    `InUse`, and nothing is written.
//! 3. **The lock is the transaction's.** It is let go at the commit or
//!    the rollback, so two namings never interleave between the look
//!    and the write, and a naming that fails leaves nothing held.
//! 4. **No name, no lock.** A create with no name and an edit that
//!    leaves the name as it is cannot collide, and take nothing.
//!
//! Each table's own `UNIQUE` stays, so a write that did not follow
//! these rules still cannot give one family a name twice.

use sqlx::{PgConnection, Row as _};

use crate::store::Error;

/// The advisory lock every naming of a tool is serialized on: one
/// fixed key, `"toolname"` in ASCII, held for the transaction.
const LOCK: i64 = 0x746f_6f6c_6e61_6d65;

/// Rules 1 and 3: take the names lock for the caller's transaction,
/// waiting for any other naming to commit or roll back.
pub async fn lock(conn: &mut PgConnection) -> Result<(), Error> {
    sqlx::query("SELECT pg_advisory_xact_lock($1)")
        .bind(LOCK)
        .execute(&mut *conn)
        .await?;
    Ok(())
}

/// Rule 2: whether a standalone or a connected tool is called `name`.
pub async fn taken(conn: &mut PgConnection, name: &str) -> Result<bool, Error> {
    let row = sqlx::query(
        "SELECT EXISTS (SELECT 1 FROM diverge.tools WHERE name = $1) \
         OR EXISTS (SELECT 1 FROM diverge.connected_standalone_tools WHERE name = $1) AS taken",
    )
    .bind(name)
    .fetch_one(&mut *conn)
    .await?;
    Ok(row.try_get("taken")?)
}

/// Rules 1, 2 and 4 at once: for a naming to `name` — `None`, or the
/// name the tool has already, `current`, takes nothing — the lock
/// taken and whether the name is held by any tool. `false` is free to
/// write.
pub async fn claim(conn: &mut PgConnection, name: Option<&str>, current: Option<&str>) -> Result<bool, Error> {
    let Some(name) = name else {
        return Ok(false);
    };
    if current == Some(name) {
        return Ok(false);
    }
    lock(conn).await?;
    taken(conn, name).await
}
