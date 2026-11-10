//! Messages: adding one, and reading a conversation's.

use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::{PgPool, Postgres, Row as _, Transaction};

use crate::caller::Participant;
use crate::failure::Failure;

/// One message, as a reader sees it.
#[derive(Debug, Clone, Serialize)]
pub struct Message {
    /// The message's id, rising through the conversation.
    pub id: i64,
    /// Who sent it, as the daemon attested them.
    pub from: Participant,
    /// What it says.
    pub text: String,
    /// When it was sent.
    pub sent: DateTime<Utc>,
}

/// Add `text` from `sender` to conversation `id`, inside the send's
/// transaction, and move the sender's own read mark past it; the
/// message's id.
pub async fn add(tx: &mut Transaction<'_, Postgres>, id: i64, sender: &Participant, text: &str) -> Result<i64, Failure> {
    let row = sqlx::query(
        "insert into courier_messages (conversation, sender_template, sender_index, sender_name, text)
         values ($1, $2, $3, $4, $5) returning id",
    )
    .bind(id)
    .bind(&sender.template)
    .bind(sender.index)
    .bind(&sender.name)
    .bind(text)
    .fetch_one(&mut **tx)
    .await?;
    let message: i64 = row.try_get("id")?;
    mark(&mut **tx, id, sender, message).await?;
    Ok(message)
}

/// Conversation `id`'s messages for `reader`, oldest first: only those
/// the reader has not read, unless `include_read`. Every message
/// returned is then read for the reader. The caller has already been
/// checked to be in the conversation.
pub async fn read(pool: &PgPool, id: i64, reader: &Participant, include_read: bool) -> Result<Vec<Message>, Failure> {
    let rows = sqlx::query(
        "select m.id, m.sender_template, m.sender_index, m.sender_name, m.text, m.sent
         from courier_messages m
         left join courier_reads r on r.conversation = m.conversation and r.reader_template = $2 and r.reader_index = $3
         where m.conversation = $1 and ($4 or m.id > coalesce(r.through, 0))
         order by m.id",
    )
    .bind(id)
    .bind(&reader.template)
    .bind(reader.index)
    .bind(include_read)
    .fetch_all(pool)
    .await?;
    let mut messages = Vec::with_capacity(rows.len());
    for row in rows {
        messages.push(Message {
            id: row.try_get("id")?,
            from: Participant { template: row.try_get("sender_template")?, index: row.try_get("sender_index")?, name: row.try_get("sender_name")? },
            text: row.try_get("text")?,
            sent: row.try_get("sent")?,
        });
    }
    if let Some(last) = messages.last() {
        mark(pool, id, reader, last.id).await?;
    }
    Ok(messages)
}

/// Move `reader`'s read mark in conversation `id` to `through`, never
/// back.
async fn mark<'e, E: sqlx::PgExecutor<'e>>(executor: E, id: i64, reader: &Participant, through: i64) -> Result<(), Failure> {
    sqlx::query(
        "insert into courier_reads (conversation, reader_template, reader_index, through) values ($1, $2, $3, $4)
         on conflict (conversation, reader_template, reader_index)
         do update set through = greatest(courier_reads.through, excluded.through)",
    )
    .bind(id)
    .bind(&reader.template)
    .bind(reader.index)
    .bind(through)
    .execute(executor)
    .await?;
    Ok(())
}
