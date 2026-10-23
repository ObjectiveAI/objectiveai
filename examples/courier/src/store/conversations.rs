//! Conversations: opening one, and listing a caller's.

use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::{PgPool, Postgres, Row as _, Transaction};

use crate::caller::Participant;
use crate::failure::Failure;

/// One conversation, as the caller sees it in their inbox.
#[derive(Debug, Clone, Serialize)]
pub struct Listed {
    /// The conversation's id.
    pub id: i64,
    /// The topic it was opened on.
    pub topic: String,
    /// The other agent in it.
    pub with: Participant,
    /// How many of its messages the caller has not read.
    pub unread: i64,
    /// When its latest message was sent.
    pub last: DateTime<Utc>,
}

/// Open a conversation between `from` and `to` on `topic`, inside the
/// send's transaction; its id.
pub async fn open(tx: &mut Transaction<'_, Postgres>, template: &str, topic: &str, from: &Participant, to: &Participant) -> Result<i64, Failure> {
    let row = sqlx::query(
        "insert into courier_conversations (template, topic, a_template, a_index, a_name, b_template, b_index, b_name)
         values ($1, $2, $3, $4, $5, $6, $7, $8) returning id",
    )
    .bind(template)
    .bind(topic)
    .bind(&from.template)
    .bind(from.index)
    .bind(&from.name)
    .bind(&to.template)
    .bind(to.index)
    .bind(&to.name)
    .fetch_one(&mut **tx)
    .await?;
    Ok(row.try_get("id")?)
}

/// The other participant of conversation `id`, when `caller` is in
/// it; refused as [`Failure::NoConversation`] when the conversation
/// does not exist, is another template's, or the caller is not in it.
pub async fn other(pool: &PgPool, template: &str, id: i64, caller: &Participant) -> Result<Participant, Failure> {
    let row = sqlx::query("select a_template, a_index, a_name, b_template, b_index, b_name from courier_conversations where id = $1 and template = $2")
        .bind(id)
        .bind(template)
        .fetch_optional(pool)
        .await?
        .ok_or(Failure::NoConversation(id))?;
    let a = Participant { template: row.try_get("a_template")?, index: row.try_get("a_index")?, name: row.try_get("a_name")? };
    let b = Participant { template: row.try_get("b_template")?, index: row.try_get("b_index")?, name: row.try_get("b_name")? };
    if a.is(&caller.template, caller.index) {
        Ok(b)
    } else if b.is(&caller.template, caller.index) {
        Ok(a)
    } else {
        Err(Failure::NoConversation(id))
    }
}

/// The caller's conversations, newest message first: only those with
/// a message the caller has not read, unless `include_read`.
pub async fn list(pool: &PgPool, template: &str, caller: &Participant, include_read: bool) -> Result<Vec<Listed>, Failure> {
    let rows = sqlx::query(
        "select c.id, c.topic, c.a_template, c.a_index, c.a_name, c.b_template, c.b_index, c.b_name,
                count(m.id) filter (where m.id > coalesce(r.through, 0)) as unread,
                max(m.sent) as last
         from courier_conversations c
         join courier_messages m on m.conversation = c.id
         left join courier_reads r on r.conversation = c.id and r.reader_template = $2 and r.reader_index = $3
         where c.template = $1
           and ((c.a_template = $2 and c.a_index = $3) or (c.b_template = $2 and c.b_index = $3))
         group by c.id, r.through
         having $4 or count(m.id) filter (where m.id > coalesce(r.through, 0)) > 0
         order by last desc",
    )
    .bind(template)
    .bind(&caller.template)
    .bind(caller.index)
    .bind(include_read)
    .fetch_all(pool)
    .await?;
    let mut listed = Vec::with_capacity(rows.len());
    for row in rows {
        let a = Participant { template: row.try_get("a_template")?, index: row.try_get("a_index")?, name: row.try_get("a_name")? };
        let b = Participant { template: row.try_get("b_template")?, index: row.try_get("b_index")?, name: row.try_get("b_name")? };
        let with = if a.is(&caller.template, caller.index) { b } else { a };
        listed.push(Listed { id: row.try_get("id")?, topic: row.try_get("topic")?, with, unread: row.try_get("unread")?, last: row.try_get("last")? });
    }
    Ok(listed)
}
