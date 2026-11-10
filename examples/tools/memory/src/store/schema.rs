//! The table, made where absent.

use sqlx::PgPool;

/// Every statement that makes the notes' table, in order. Each is
/// `if not exists`, so ensuring it again is nothing. `search` is the
/// text's English full-text vector, kept by Postgres itself.
const STATEMENTS: [&str; 3] = [
    "create table if not exists memory_notes (
        id bigserial primary key,
        text text not null,
        tags text[] not null default '{}',
        made timestamptz not null default now(),
        search tsvector generated always as (to_tsvector('english', text)) stored
    )",
    "create index if not exists memory_notes_by_search on memory_notes using gin (search)",
    "create index if not exists memory_notes_by_tags on memory_notes using gin (tags)",
];

/// Make the table where absent.
pub async fn ensure(pool: &PgPool) -> Result<(), sqlx::Error> {
    for statement in STATEMENTS {
        sqlx::query(statement).execute(pool).await?;
    }
    Ok(())
}
