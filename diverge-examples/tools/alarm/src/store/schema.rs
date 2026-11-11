//! The table, made where absent.

use sqlx::PgPool;

/// Every statement that makes the alarms' table, in order. Each is
/// `if not exists`, so ensuring it again is nothing. `cron` is null
/// for a one-shot alarm; `due` is when it next fires.
const STATEMENTS: [&str; 3] = [
    "create table if not exists alarm_alarms (
        id bigserial primary key,
        agent_template text not null,
        agent_index bigint not null,
        agent_name text,
        note text not null,
        cron text,
        due timestamptz not null,
        made timestamptz not null default now()
    )",
    "create index if not exists alarm_alarms_by_due on alarm_alarms (due)",
    "create index if not exists alarm_alarms_by_agent on alarm_alarms (agent_template, agent_index)",
];

/// Make the table where absent.
pub async fn ensure(pool: &PgPool) -> Result<(), sqlx::Error> {
    for statement in STATEMENTS {
        sqlx::query(statement).execute(pool).await?;
    }
    Ok(())
}
