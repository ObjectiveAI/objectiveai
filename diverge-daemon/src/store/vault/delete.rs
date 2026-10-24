//! Removing a vault key, or one of its values.

use diverge_sdk::daemon::creator::Creator;
use sqlx::types::Json;
use sqlx::{PgConnection, Row as _};

use crate::store::{Error, in_use};

/// What a delete came to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Deleted {
    /// The key, or the slot, gone.
    Deleted,
    /// The key has no slot by that number; nothing changed.
    NotFound,
    /// An agent or a tool binds the key, which is left as it is;
    /// nothing changed.
    InUse,
    /// A running container holds the slot named — or, for the whole
    /// key, one of its slots — which is left as it is; nothing
    /// changed.
    Locked,
    /// The slot named is the key's only one: the key is what to
    /// delete; nothing changed.
    Last,
}

/// Delete the key, or one slot of it. The rules, in order, for the
/// key — no slot named:
///
/// 1. A record binds it: `InUse`.
/// 2. Any slot of it is held: `Locked`. Every slot's row is locked
///    for the transaction first, so a container taking one this
///    instant is either seen as its holder or waits.
/// 3. Else the key and its slots gone: `Deleted`.
///
/// And for a slot:
///
/// 1. The key has no slot by that number: `NotFound`.
/// 2. A container holds it: `Locked`, the rows locked as above.
/// 3. It is the key's only slot: `Last`.
/// 4. Else the slot gone: `Deleted`.
///
/// The caller holds the key's row locked, so a binding made between
/// the look and the delete waits on the same row the other way: a
/// create or an edit that binds a key reads it under the same
/// transaction first. A key the vault does not hold is the caller's
/// to say before this.
pub async fn delete(conn: &mut PgConnection, key: &str, slot: Option<u32>) -> Result<Deleted, Error> {
    if slot.is_none() && in_use::vault_key(conn, key).await? {
        return Ok(Deleted::InUse);
    }
    let rows = sqlx::query("SELECT slot, holder FROM diverge.vault_slots WHERE key = $1 ORDER BY slot FOR UPDATE")
        .bind(key)
        .fetch_all(&mut *conn)
        .await?;
    let mut slots = Vec::with_capacity(rows.len());
    for row in &rows {
        let number: i32 = row.try_get("slot")?;
        let holder: Option<Json<Creator>> = row.try_get("holder")?;
        slots.push((u32::try_from(number).unwrap_or(0), holder.is_some()));
    }
    let Some(slot) = slot else {
        if slots.iter().any(|(_, held)| *held) {
            return Ok(Deleted::Locked);
        }
        sqlx::query("DELETE FROM diverge.vault WHERE key = $1")
            .bind(key)
            .execute(&mut *conn)
            .await?;
        return Ok(Deleted::Deleted);
    };
    let Some((_, held)) = slots.iter().find(|(number, _)| *number == slot) else {
        return Ok(Deleted::NotFound);
    };
    if *held {
        return Ok(Deleted::Locked);
    }
    if slots.len() == 1 {
        return Ok(Deleted::Last);
    }
    sqlx::query("DELETE FROM diverge.vault_slots WHERE key = $1 AND slot = $2")
        .bind(key)
        .bind(i32::try_from(slot).unwrap_or(i32::MAX))
        .execute(&mut *conn)
        .await?;
    Ok(Deleted::Deleted)
}
