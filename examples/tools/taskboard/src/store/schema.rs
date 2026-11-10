//! The tables, made where absent.

use sqlx::PgPool;

/// Every statement that makes the board's tables, in order. Each is
/// `if not exists`, so ensuring them again is nothing. A task's holder
/// is set while it is claimed and kept once it is done; it is cleared
/// when the task is released.
const STATEMENTS: [&str; 3] = [
    "create table if not exists taskboard_tasks (
        id bigserial primary key,
        title text not null,
        detail text not null,
        state text not null default 'open' check (state in ('open', 'claimed', 'done')),
        poster_template text not null,
        poster_index bigint not null,
        poster_name text,
        holder_template text,
        holder_index bigint,
        holder_name text,
        result text,
        posted timestamptz not null default now(),
        claimed timestamptz,
        done timestamptz
    )",
    "create table if not exists taskboard_comments (
        id bigserial primary key,
        task bigint not null references taskboard_tasks (id) on delete cascade,
        author_template text not null,
        author_index bigint not null,
        author_name text,
        text text not null,
        made timestamptz not null default now()
    )",
    "create index if not exists taskboard_comments_by_task on taskboard_comments (task, id)",
];

/// Make the tables where absent.
pub async fn ensure(pool: &PgPool) -> Result<(), sqlx::Error> {
    for statement in STATEMENTS {
        sqlx::query(statement).execute(pool).await?;
    }
    Ok(())
}
