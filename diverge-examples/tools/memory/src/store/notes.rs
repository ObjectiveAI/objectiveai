//! Notes: keeping one, recalling some, forgetting one.

use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::{PgPool, Row as _};

use crate::failure::Failure;

/// One note, as a recall shows it.
#[derive(Debug, Clone, Serialize)]
pub struct Note {
    /// The note's id, rising as notes are kept.
    pub id: i64,
    /// What it says.
    pub text: String,
    /// Its tags, as kept.
    pub tags: Vec<String>,
    /// When it was kept.
    pub made: DateTime<Utc>,
}

/// Keep `text` under `tags`; the note's id.
pub async fn add(pool: &PgPool, text: &str, tags: &[String]) -> Result<i64, Failure> {
    let row = sqlx::query("insert into memory_notes (text, tags) values ($1, $2) returning id").bind(text).bind(tags).fetch_one(pool).await?;
    Ok(row.try_get("id")?)
}

/// At most `limit` notes carrying every tag of `tags`. With `query`,
/// only those whose text matches it as a web search
/// (`websearch_to_tsquery`, English), best first and newest first
/// among equals; without, newest first.
pub async fn recall(pool: &PgPool, query: Option<&str>, tags: &[String], limit: i64) -> Result<Vec<Note>, Failure> {
    let rows = match query {
        Some(query) => {
            sqlx::query(
                "select id, text, tags, made from memory_notes, websearch_to_tsquery('english', $1) as q
                 where tags @> $2 and search @@ q
                 order by ts_rank(search, q) desc, id desc limit $3",
            )
            .bind(query)
            .bind(tags)
            .bind(limit)
            .fetch_all(pool)
            .await?
        }
        None => sqlx::query("select id, text, tags, made from memory_notes where tags @> $1 order by id desc limit $2").bind(tags).bind(limit).fetch_all(pool).await?,
    };
    rows.iter()
        .map(|row| Ok(Note { id: row.try_get("id")?, text: row.try_get("text")?, tags: row.try_get("tags")?, made: row.try_get("made")? }))
        .collect()
}

/// Delete note `id`; a note that is not there is [`Failure::NoNote`].
pub async fn forget(pool: &PgPool, id: i64) -> Result<(), Failure> {
    let deleted = sqlx::query("delete from memory_notes where id = $1").bind(id).execute(pool).await?.rows_affected();
    if deleted == 0 {
        return Err(Failure::NoNote(id));
    }
    Ok(())
}
