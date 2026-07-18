//! `channels logs open`: reveal one entry's content by id, scoped to
//! its channel. Pure read — opening never advances a watermark.

use objectiveai_sdk::cli::command::AgentArguments;
use sqlx::Row as _;

use super::super::{Error, Pool};
use super::{Direction, MessageContent};

/// Load one channel-log entry (envelope + content) by `(channel_id,
/// entry_id)`. `Ok(None)` when no such entry exists in that channel.
pub async fn read_content_by_id(
    pool: &Pool,
    channel_id: &str,
    entry_id: i64,
) -> Result<Option<MessageContent>, Error> {
    let Some(row) = sqlx::query(
        "SELECT id, direction, identity, content, delivered_at \
         FROM objectiveai.channel_messages \
         WHERE channel_id = $1 AND id = $2",
    )
    .bind(channel_id)
    .bind(entry_id)
    .fetch_optional(&**pool)
    .await?
    else {
        return Ok(None);
    };
    let direction_text: String = row.try_get("direction")?;
    let direction = Direction::parse(&direction_text)
        .ok_or_else(|| Error::InvalidData(format!("channel direction {direction_text:?}")))?;
    let identity: sqlx::types::Json<AgentArguments> = row.try_get("identity")?;
    let content: sqlx::types::Json<serde_json::Value> = row.try_get("content")?;
    Ok(Some(MessageContent {
        id: row.try_get("id")?,
        direction,
        identity: identity.0,
        delivered_at: row.try_get("delivered_at")?,
        content: content.0,
    }))
}
