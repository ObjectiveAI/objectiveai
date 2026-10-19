//! Inserting a value into a vault key.

use diverge_sdk::daemon::creator::Creator;
use serde_json::Value;
use sqlx::types::Json;
use sqlx::{PgConnection, Row as _};

use crate::store::Error;

/// What an insert came to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Insert {
    /// The key, made, holding the value as its slot `0`.
    Created,
    /// The key the vault held, the value appended as the slot named.
    Appended(u32),
    /// The slot named, its value replaced.
    Replaced,
    /// No such slot: the key has none by that number, or the vault has
    /// no such key; nothing changed.
    NotFound,
    /// A running container holds the slot named, which is left as it
    /// is; nothing changed.
    Locked,
}

/// Make the key hold `value`. The rules, in order:
///
/// 1. No slot named: the key is inserted with `creator` where the
///    vault had no such key, and the value is appended as its next
///    slot — `0` for a key just made, one past the highest otherwise.
///    `Created`, or `Appended` with the number.
/// 2. A slot named on a key or a slot the vault does not hold:
///    `NotFound`.
/// 3. A slot named that a container holds: `Locked`. The slot's row
///    is locked for the transaction first, so a container taking it
///    this instant is either seen as its holder or waits.
/// 4. Else the slot's value replaced: `Replaced`.
///
/// Tags, the time and the maker are as they were. The caller holds
/// the key's row locked, which serializes the appends.
pub async fn insert(conn: &mut PgConnection, key: &str, slot: Option<u32>, value: &Value, creator: &Creator) -> Result<Insert, Error> {
    let Some(slot) = slot else {
        let made = sqlx::query("INSERT INTO diverge.vault (key, creator) VALUES ($1, $2) ON CONFLICT (key) DO NOTHING RETURNING key")
            .bind(key)
            .bind(Json(creator))
            .fetch_optional(&mut *conn)
            .await?
            .is_some();
        let row = sqlx::query(
            "INSERT INTO diverge.vault_slots (key, slot, value) \
             SELECT $1::text, COALESCE(MAX(slot) + 1, 0), $2::jsonb FROM diverge.vault_slots WHERE key = $1 \
             RETURNING slot",
        )
        .bind(key)
        .bind(Json(value))
        .fetch_one(&mut *conn)
        .await?;
        let appended: i32 = row.try_get("slot")?;
        return Ok(if made { Insert::Created } else { Insert::Appended(u32::try_from(appended).unwrap_or(0)) });
    };
    let number = i32::try_from(slot).unwrap_or(i32::MAX);
    let row = sqlx::query("SELECT holder FROM diverge.vault_slots WHERE key = $1 AND slot = $2 FOR UPDATE")
        .bind(key)
        .bind(number)
        .fetch_optional(&mut *conn)
        .await?;
    let Some(row) = row else {
        return Ok(Insert::NotFound);
    };
    let holder: Option<Json<Creator>> = row.try_get("holder")?;
    if holder.is_some() {
        return Ok(Insert::Locked);
    }
    sqlx::query("UPDATE diverge.vault_slots SET value = $3 WHERE key = $1 AND slot = $2")
        .bind(key)
        .bind(number)
        .bind(Json(value))
        .execute(&mut *conn)
        .await?;
    Ok(Insert::Replaced)
}
