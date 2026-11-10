//! Comments: adding one to a task, and reading a task's.

use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::{PgPool, Row as _};

use super::{participant, tasks};
use crate::caller::Participant;
use crate::failure::Failure;

/// One comment, as a reader sees it.
#[derive(Debug, Clone, Serialize)]
pub struct Comment {
    /// The comment's id, rising as comments are made.
    pub id: i64,
    /// Who made it, as the daemon attested them.
    pub from: Participant,
    /// What it says.
    pub text: String,
    /// When it was made.
    pub made: DateTime<Utc>,
}

/// Add `text` from `author` to task `task`; the comment's id. The
/// insert reads the task in the same statement, so a task that is not
/// there takes no comment.
pub async fn create(pool: &PgPool, task: i64, author: &Participant, text: &str) -> Result<i64, Failure> {
    let row = sqlx::query(
        "insert into taskboard_comments (task, author_template, author_index, author_name, text)
         select id, $2, $3, $4, $5 from taskboard_tasks where id = $1 returning id",
    )
    .bind(task)
    .bind(&author.template)
    .bind(author.index)
    .bind(&author.name)
    .bind(text)
    .fetch_optional(pool)
    .await?;
    match row {
        Some(row) => Ok(row.try_get("id")?),
        None => Err(Failure::NoTask(task)),
    }
}

/// Task `task`'s comments, oldest first.
pub async fn list(pool: &PgPool, task: i64) -> Result<Vec<Comment>, Failure> {
    if !tasks::exists(pool, task).await? {
        return Err(Failure::NoTask(task));
    }
    let rows = sqlx::query("select id, author_template, author_index, author_name, text, made from taskboard_comments where task = $1 order by id")
        .bind(task)
        .fetch_all(pool)
        .await?;
    let comment = |row: &sqlx::postgres::PgRow| -> Result<Comment, sqlx::Error> {
        Ok(Comment {
            id: row.try_get("id")?,
            from: participant(row, "author")?.ok_or_else(|| sqlx::Error::Decode("a comment with no author".into()))?,
            text: row.try_get("text")?,
            made: row.try_get("made")?,
        })
    };
    Ok(rows.iter().map(comment).collect::<Result<_, _>>()?)
}
