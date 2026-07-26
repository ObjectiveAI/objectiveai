//! Channel reads: authenticate a secret, list the log (all / pending),
//! and the existence probe subscribe uses.

use objectiveai_sdk::identity::Identity;
use sqlx::Row as _;

use super::super::{Error, Pool};
use super::{ChannelAuth, ChannelState, Direction, MessageEnvelope, Role};

/// The `(directions to READ, watermark column)` a role paginates
/// over: a publisher reads REPLIES against `pub_read_index`; an owner
/// reads REQUESTS — plus the accept-time `publish` seed (the offer is
/// the owner's first unread entry) — against `owner_read_index`. The
/// `publish_message` row enumerates for NEITHER role (it surfaces as
/// the publish item's `message_id`, not as its own entry; its id may
/// permanently sit above the owner watermark — harmless, it matches
/// no read filter). The column name is one of two hardcoded
/// identifiers (never user input), safe to interpolate.
fn role_read_params(role: Role) -> (&'static [&'static str], &'static str) {
    match role {
        Role::Publisher => (&["reply"], "pub_read_index"),
        Role::Owner => (&["request", "publish"], "owner_read_index"),
    }
}

/// Fetch a channel's authentication + state snapshot. `Ok(None)` when
/// no channel with that id exists.
pub async fn channel_auth(pool: &Pool, channel_id: &str) -> Result<Option<ChannelAuth>, Error> {
    let Some(row) = sqlx::query(
        "SELECT pub_secret, owner_secret, state \
         FROM objectiveai.channels WHERE id = $1",
    )
    .bind(channel_id)
    .fetch_optional(&**pool)
    .await?
    else {
        return Ok(None);
    };
    let state_text: String = row.try_get("state")?;
    let state = ChannelState::parse(&state_text)
        .ok_or_else(|| Error::InvalidData(format!("channel state {state_text:?}")))?;
    Ok(Some(ChannelAuth {
        pub_secret: row.try_get("pub_secret")?,
        owner_secret: row.try_get("owner_secret")?,
        state,
    }))
}

/// Decode a `channel_messages` envelope row (no content).
fn envelope_of(row: &sqlx::postgres::PgRow) -> Result<MessageEnvelope, Error> {
    let direction_text: String = row.try_get("direction")?;
    let direction = Direction::parse(&direction_text)
        .ok_or_else(|| Error::InvalidData(format!("channel direction {direction_text:?}")))?;
    let identity: sqlx::types::Json<Identity> = row.try_get("identity")?;
    Ok(MessageEnvelope {
        id: row.try_get("id")?,
        direction,
        identity: identity.0,
        delivered_at: row.try_get("delivered_at")?,
    })
}

/// `logs list --all`: every message envelope in the channel, ascending
/// by id, `id > after_id` (exclusive), capped by `limit`. Pure read —
/// no watermark bump. The `publish_message` seed never enumerates (it
/// rides the publish item as `message_id`).
pub async fn read_all(
    pool: &Pool,
    channel_id: &str,
    after_id: Option<i64>,
    limit: Option<i64>,
) -> Result<Vec<MessageEnvelope>, Error> {
    let rows = sqlx::query(
        "SELECT id, direction, identity, delivered_at \
         FROM objectiveai.channel_messages \
         WHERE channel_id = $1 AND id > COALESCE($2, 0) \
           AND direction <> 'publish_message' \
         ORDER BY id ASC LIMIT $3",
    )
    .bind(channel_id)
    .bind(after_id)
    .bind(limit)
    .fetch_all(&**pool)
    .await?;
    rows.iter().map(envelope_of).collect()
}

/// `logs list --pending` / the drain half of `subscribe`: the envelopes
/// this ROLE hasn't read (messages from the OTHER side past its
/// watermark), ascending by id, capped by `limit`. Advances the role's
/// watermark monotonically to the max id returned (a `GREATEST` bump,
/// never a downgrade), atomically with the read.
pub async fn read_pending(
    pool: &Pool,
    channel_id: &str,
    role: Role,
    after_id: Option<i64>,
    limit: Option<i64>,
) -> Result<Vec<MessageEnvelope>, Error> {
    let (directions, watermark) = role_read_params(role);
    // The watermark column is a hardcoded identifier; the direction
    // set is a bound array param. sel drains, bump advances the
    // watermark by the max drained id (NULL when sel is empty →
    // GREATEST leaves it be).
    let sql = format!(
        "WITH sel AS ( \
            SELECT m.id, m.direction, m.identity, m.delivered_at \
            FROM objectiveai.channel_messages m \
            JOIN objectiveai.channels c ON c.id = m.channel_id \
            WHERE m.channel_id = $1 AND m.direction = ANY($2) \
              AND m.id > GREATEST(c.{watermark}, COALESCE($3, 0)) \
            ORDER BY m.id ASC LIMIT $4 \
         ), bump AS ( \
            UPDATE objectiveai.channels \
            SET {watermark} = GREATEST({watermark}, (SELECT MAX(id) FROM sel)) \
            WHERE id = $1 \
         ) \
         SELECT id, direction, identity, delivered_at FROM sel ORDER BY id ASC"
    );
    let rows = sqlx::query(&sql)
        .bind(channel_id)
        .bind(directions)
        .bind(after_id)
        .bind(limit)
        .fetch_all(&**pool)
        .await?;
    rows.iter().map(envelope_of).collect()
}

/// Side-effect-free existence probe: does this role have ANY unread
/// message (from the other side, past its watermark)? Drives
/// subscribe's immediate-return path.
pub async fn any_pending(
    pool: &Pool,
    channel_id: &str,
    role: Role,
    after_id: Option<i64>,
) -> Result<bool, Error> {
    let (directions, watermark) = role_read_params(role);
    let sql = format!(
        "SELECT EXISTS( \
            SELECT 1 FROM objectiveai.channel_messages m \
            JOIN objectiveai.channels c ON c.id = m.channel_id \
            WHERE m.channel_id = $1 AND m.direction = ANY($2) \
              AND m.id > GREATEST(c.{watermark}, COALESCE($3, 0)) \
         )"
    );
    let exists: bool = sqlx::query_scalar(&sql)
        .bind(channel_id)
        .bind(directions)
        .bind(after_id)
        .fetch_one(&**pool)
        .await?;
    Ok(exists)
}

/// The channel's `publish_message` seed-row id — the wire publish
/// item's `message_id`. One per channel by construction; `None` only
/// for a pre-seed legacy channel.
pub async fn publish_message_id(pool: &Pool, channel_id: &str) -> Result<Option<i64>, Error> {
    Ok(sqlx::query_scalar(
        "SELECT id FROM objectiveai.channel_messages \
         WHERE channel_id = $1 AND direction = 'publish_message'",
    )
    .bind(channel_id)
    .fetch_optional(&**pool)
    .await?)
}

/// The channel's current lifecycle state, or `None` if it doesn't
/// exist. Used by subscribe to detect a close it must return on.
pub async fn channel_state(pool: &Pool, channel_id: &str) -> Result<Option<ChannelState>, Error> {
    let state_text: Option<String> = sqlx::query_scalar(
        "SELECT state FROM objectiveai.channels WHERE id = $1",
    )
    .bind(channel_id)
    .fetch_optional(&**pool)
    .await?;
    state_text
        .map(|s| {
            ChannelState::parse(&s)
                .ok_or_else(|| Error::InvalidData(format!("channel state {s:?}")))
        })
        .transpose()
}
