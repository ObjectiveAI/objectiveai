//! Tasks: posting, listing, and every change of state.
//!
//! The rules, each one statement's condition:
//! - an open task is claimed by the caller, who becomes its holder;
//! - a claimed task is completed, with its result, only by its holder;
//! - a claimed task is released back to open only by its holder, its
//!   holder cleared;
//! - a task, in any state, is deleted only by its poster, its comments
//!   with it.
//!
//! A statement that changes nothing is followed by a look at the task,
//! to say whether it is absent or the condition failed.

use chrono::{DateTime, Utc};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use sqlx::{PgPool, Row as _};

use super::participant;
use crate::caller::Participant;
use crate::failure::Failure;

/// Where a task stands.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum State {
    /// Posted, held by no one.
    Open,
    /// Held by the agent that claimed it.
    Claimed,
    /// Completed by its holder, with a result.
    Done,
}

impl State {
    /// The state as the table keeps it.
    pub fn as_str(self) -> &'static str {
        match self {
            State::Open => "open",
            State::Claimed => "claimed",
            State::Done => "done",
        }
    }

    /// The state the table keeps as `text`.
    fn parse(text: &str) -> Result<State, sqlx::Error> {
        match text {
            "open" => Ok(State::Open),
            "claimed" => Ok(State::Claimed),
            "done" => Ok(State::Done),
            other => Err(sqlx::Error::Decode(format!("no task state {other}").into())),
        }
    }
}

/// One task, as the board shows it.
#[derive(Debug, Clone, Serialize)]
pub struct Task {
    /// The task's id, rising as tasks are posted.
    pub id: i64,
    /// What the task is, in a line.
    pub title: String,
    /// What the task asks, in full.
    pub detail: String,
    /// Where it stands.
    pub state: State,
    /// The agent that posted it.
    pub poster: Participant,
    /// The agent that claimed it: present while claimed, and kept once
    /// done.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub holder: Option<Participant>,
    /// What its holder answered on completing it.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<String>,
    /// When it was posted.
    pub posted: DateTime<Utc>,
    /// When it was last claimed, while it is claimed or done.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub claimed: Option<DateTime<Utc>>,
    /// When it was completed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub done: Option<DateTime<Utc>>,
    /// How many comments it has.
    pub comments: i64,
}

/// Post a task from `poster`; its id.
pub async fn create(pool: &PgPool, title: &str, detail: &str, poster: &Participant) -> Result<i64, Failure> {
    let row = sqlx::query("insert into taskboard_tasks (title, detail, poster_template, poster_index, poster_name) values ($1, $2, $3, $4, $5) returning id")
        .bind(title)
        .bind(detail)
        .bind(&poster.template)
        .bind(poster.index)
        .bind(&poster.name)
        .fetch_one(pool)
        .await?;
    Ok(row.try_get("id")?)
}

/// At most `limit` tasks, oldest first: every task, or those in
/// `state`.
pub async fn list(pool: &PgPool, state: Option<State>, limit: i64) -> Result<Vec<Task>, Failure> {
    let rows = sqlx::query(
        "select t.*, (select count(*) from taskboard_comments c where c.task = t.id) as comments
         from taskboard_tasks t where $1::text is null or t.state = $1 order by t.id limit $2",
    )
    .bind(state.map(State::as_str))
    .bind(limit)
    .fetch_all(pool)
    .await?;
    let task = |row: &sqlx::postgres::PgRow| -> Result<Task, sqlx::Error> {
        Ok(Task {
            id: row.try_get("id")?,
            title: row.try_get("title")?,
            detail: row.try_get("detail")?,
            state: State::parse(row.try_get("state")?)?,
            poster: participant(row, "poster")?.ok_or_else(|| sqlx::Error::Decode("a task with no poster".into()))?,
            holder: participant(row, "holder")?,
            result: row.try_get("result")?,
            posted: row.try_get("posted")?,
            claimed: row.try_get("claimed")?,
            done: row.try_get("done")?,
            comments: row.try_get("comments")?,
        })
    };
    Ok(rows.iter().map(task).collect::<Result<_, _>>()?)
}

/// Claim open task `id` for `caller`.
pub async fn claim(pool: &PgPool, id: i64, caller: &Participant) -> Result<(), Failure> {
    let changed = sqlx::query(
        "update taskboard_tasks set state = 'claimed', holder_template = $2, holder_index = $3, holder_name = $4, claimed = now()
         where id = $1 and state = 'open'",
    )
    .bind(id)
    .bind(&caller.template)
    .bind(caller.index)
    .bind(&caller.name)
    .execute(pool)
    .await?
    .rows_affected();
    unchanged(pool, id, changed, Failure::NotOpen(id)).await
}

/// Complete task `id`, held by `caller`, with `result`.
pub async fn complete(pool: &PgPool, id: i64, caller: &Participant, result: &str) -> Result<(), Failure> {
    let changed = sqlx::query(
        "update taskboard_tasks set state = 'done', result = $4, done = now()
         where id = $1 and state = 'claimed' and holder_template = $2 and holder_index = $3",
    )
    .bind(id)
    .bind(&caller.template)
    .bind(caller.index)
    .bind(result)
    .execute(pool)
    .await?
    .rows_affected();
    unchanged(pool, id, changed, Failure::NotHolder(id)).await
}

/// Release task `id`, held by `caller`, back to open.
pub async fn release(pool: &PgPool, id: i64, caller: &Participant) -> Result<(), Failure> {
    let changed = sqlx::query(
        "update taskboard_tasks set state = 'open', holder_template = null, holder_index = null, holder_name = null, claimed = null
         where id = $1 and state = 'claimed' and holder_template = $2 and holder_index = $3",
    )
    .bind(id)
    .bind(&caller.template)
    .bind(caller.index)
    .execute(pool)
    .await?
    .rows_affected();
    unchanged(pool, id, changed, Failure::NotHolder(id)).await
}

/// Delete task `id`, posted by `caller`, and its comments.
pub async fn delete(pool: &PgPool, id: i64, caller: &Participant) -> Result<(), Failure> {
    let changed = sqlx::query("delete from taskboard_tasks where id = $1 and poster_template = $2 and poster_index = $3")
        .bind(id)
        .bind(&caller.template)
        .bind(caller.index)
        .execute(pool)
        .await?
        .rows_affected();
    unchanged(pool, id, changed, Failure::NotPoster(id)).await
}

/// Whether task `id` is on the board.
pub async fn exists(pool: &PgPool, id: i64) -> Result<bool, Failure> {
    Ok(sqlx::query("select 1 from taskboard_tasks where id = $1").bind(id).fetch_optional(pool).await?.is_some())
}

/// A change of `changed` rows: one is done; none is `refused` when
/// the task is there, and [`Failure::NoTask`] when it is not.
async fn unchanged(pool: &PgPool, id: i64, changed: u64, refused: Failure) -> Result<(), Failure> {
    if changed > 0 {
        return Ok(());
    }
    match exists(pool, id).await? {
        true => Err(refused),
        false => Err(Failure::NoTask(id)),
    }
}
