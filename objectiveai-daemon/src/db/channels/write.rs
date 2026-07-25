//! Channel writes: create a channel on accept, append messages,
//! close on owner-drop.

use objectiveai_sdk::identity::Identity;

use super::super::{Error, Pool};
use super::Direction;

/// The plugin coordinate that originated a channel (unspoofable), or
/// all-`None` when the publisher wasn't a plugin.
pub struct PluginOrigin<'a> {
    pub owner: Option<&'a str>,
    pub name: Option<&'a str>,
    pub version: Option<&'a str>,
}

/// Insert a newly-accepted channel (`state = open`). Called once, at
/// accept time — the offer that preceded it was never persisted.
#[allow(clippy::too_many_arguments)]
pub async fn insert_channel(
    pool: &Pool,
    id: &str,
    pub_secret: &str,
    owner_secret: &str,
    key: &str,
    details: &serde_json::Value,
    message: &str,
    plugin: &PluginOrigin<'_>,
    identity: &Identity,
) -> Result<(), Error> {
    let now = chrono::Utc::now().timestamp();
    sqlx::query(
        "INSERT INTO objectiveai.channels \
         (id, pub_secret, owner_secret, state, key, details, message, \
          plugin_owner, plugin_name, plugin_version, identity, \
          pub_read_index, owner_read_index, created_at) \
         VALUES ($1, $2, $3, 'open', $4, $5, $6, $7, $8, $9, $10, 0, 0, $11)",
    )
    .bind(id)
    .bind(pub_secret)
    .bind(owner_secret)
    .bind(key)
    .bind(sqlx::types::Json(details))
    .bind(message)
    .bind(plugin.owner)
    .bind(plugin.name)
    .bind(plugin.version)
    .bind(sqlx::types::Json(identity))
    .bind(now)
    .execute(&**pool)
    .await?;
    Ok(())
}

/// Append one message to a channel's log. Returns the new
/// `(id, delivered_at)` (delivered_at is unix-seconds). The caller is
/// responsible for having authorized the write (role + open state).
pub async fn insert_message(
    pool: &Pool,
    channel_id: &str,
    direction: Direction,
    identity: &Identity,
    content: &serde_json::Value,
) -> Result<(i64, i64), Error> {
    let now = chrono::Utc::now().timestamp();
    let id: i64 = sqlx::query_scalar(
        "INSERT INTO objectiveai.channel_messages \
         (channel_id, direction, identity, content, delivered_at) \
         VALUES ($1, $2, $3, $4, $5) RETURNING id",
    )
    .bind(channel_id)
    .bind(direction.as_str())
    .bind(sqlx::types::Json(identity))
    .bind(sqlx::types::Json(content))
    .bind(now)
    .fetch_one(&**pool)
    .await?;
    Ok((id, now))
}

/// Mark a channel `closed` (terminal — no further requests/replies).
/// Idempotent: a no-op on an already-closed channel or an unknown id.
/// The state-change NOTIFY wakes any blocked subscriber.
pub async fn close_channel(pool: &Pool, channel_id: &str) -> Result<(), Error> {
    let now = chrono::Utc::now().timestamp();
    sqlx::query(
        "UPDATE objectiveai.channels SET state = 'closed', closed_at = $2 \
         WHERE id = $1 AND state <> 'closed'",
    )
    .bind(channel_id)
    .bind(now)
    .execute(&**pool)
    .await?;
    Ok(())
}
