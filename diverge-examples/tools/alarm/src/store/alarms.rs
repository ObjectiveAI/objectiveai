//! Alarms: setting, listing, cancelling, and the scheduler's two.

use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::postgres::PgRow;
use sqlx::{PgPool, Postgres, Row as _, Transaction};

use crate::caller::Participant;
use crate::failure::Failure;

/// One alarm.
#[derive(Debug, Clone, Serialize)]
pub struct Alarm {
    /// The alarm's id.
    pub id: i64,
    /// The agent it wakes: the one that set it. Never shown, since an
    /// agent is only ever shown its own.
    #[serde(skip)]
    pub agent: Participant,
    /// What it says when it fires.
    pub note: String,
    /// Its pattern, when it recurs.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cron: Option<String>,
    /// When it next fires.
    pub due: DateTime<Utc>,
    /// When it was set.
    pub made: DateTime<Utc>,
}

/// The columns an [`Alarm`] is read from, in every select.
const COLUMNS: &str = "id, agent_template, agent_index, agent_name, note, cron, due, made";

/// The alarm in `row`.
fn alarm(row: &PgRow) -> Result<Alarm, sqlx::Error> {
    Ok(Alarm {
        id: row.try_get("id")?,
        agent: Participant { template: row.try_get("agent_template")?, index: row.try_get("agent_index")?, name: row.try_get("agent_name")? },
        note: row.try_get("note")?,
        cron: row.try_get("cron")?,
        due: row.try_get("due")?,
        made: row.try_get("made")?,
    })
}

/// Set an alarm for `agent`, due at `due`, recurring on `cron` if
/// given; its id.
pub async fn create(pool: &PgPool, agent: &Participant, note: &str, cron: Option<&str>, due: DateTime<Utc>) -> Result<i64, Failure> {
    let row = sqlx::query("insert into alarm_alarms (agent_template, agent_index, agent_name, note, cron, due) values ($1, $2, $3, $4, $5, $6) returning id")
        .bind(&agent.template)
        .bind(agent.index)
        .bind(&agent.name)
        .bind(note)
        .bind(cron)
        .bind(due)
        .fetch_one(pool)
        .await?;
    Ok(row.try_get("id")?)
}

/// `agent`'s alarms, soonest first.
pub async fn list(pool: &PgPool, agent: &Participant) -> Result<Vec<Alarm>, Failure> {
    let rows = sqlx::query(&format!("select {COLUMNS} from alarm_alarms where agent_template = $1 and agent_index = $2 order by due, id"))
        .bind(&agent.template)
        .bind(agent.index)
        .fetch_all(pool)
        .await?;
    Ok(rows.iter().map(alarm).collect::<Result<_, _>>()?)
}

/// Cancel `agent`'s alarm `id`. Another agent's alarm is
/// [`Failure::NoAlarm`] as an absent one is, so none is revealed.
pub async fn cancel(pool: &PgPool, id: i64, agent: &Participant) -> Result<(), Failure> {
    let deleted = sqlx::query("delete from alarm_alarms where id = $1 and agent_template = $2 and agent_index = $3")
        .bind(id)
        .bind(&agent.template)
        .bind(agent.index)
        .execute(pool)
        .await?
        .rows_affected();
    if deleted == 0 {
        return Err(Failure::NoAlarm(id));
    }
    Ok(())
}

/// Every alarm due by `now`, soonest first, its row held by the
/// transaction until it commits; rows another transaction holds are
/// skipped, so no alarm is fired twice at once.
pub async fn due(transaction: &mut Transaction<'_, Postgres>, now: DateTime<Utc>) -> Result<Vec<Alarm>, Failure> {
    let rows = sqlx::query(&format!("select {COLUMNS} from alarm_alarms where due <= $1 order by due, id for update skip locked"))
        .bind(now)
        .fetch_all(&mut **transaction)
        .await?;
    Ok(rows.iter().map(alarm).collect::<Result<_, _>>()?)
}

/// A fired alarm settled: moved to `next`, or deleted when there is
/// no next.
pub async fn settle(transaction: &mut Transaction<'_, Postgres>, id: i64, next: Option<DateTime<Utc>>) -> Result<(), Failure> {
    match next {
        Some(next) => sqlx::query("update alarm_alarms set due = $2 where id = $1").bind(id).bind(next).execute(&mut **transaction).await?,
        None => sqlx::query("delete from alarm_alarms where id = $1").bind(id).execute(&mut **transaction).await?,
    };
    Ok(())
}

/// When the soonest alarm is due, if there is any.
pub async fn next(pool: &PgPool) -> Result<Option<DateTime<Utc>>, Failure> {
    let row = sqlx::query("select min(due) as next from alarm_alarms").fetch_one(pool).await?;
    Ok(row.try_get("next")?)
}
