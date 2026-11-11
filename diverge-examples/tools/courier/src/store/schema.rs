//! The tables, made where absent.

use sqlx::PgPool;

/// Every statement that makes the courier's tables, in order. Each is
/// `if not exists`, so ensuring them again is nothing.
const STATEMENTS: [&str; 4] = [
    "create table if not exists courier_conversations (
        id bigserial primary key,
        template text not null,
        topic text not null,
        a_template text not null,
        a_index bigint not null,
        a_name text,
        b_template text not null,
        b_index bigint not null,
        b_name text,
        opened timestamptz not null default now()
    )",
    "create table if not exists courier_messages (
        id bigserial primary key,
        conversation bigint not null references courier_conversations (id),
        sender_template text not null,
        sender_index bigint not null,
        sender_name text,
        text text not null,
        sent timestamptz not null default now()
    )",
    "create index if not exists courier_messages_by_conversation on courier_messages (conversation, id)",
    "create table if not exists courier_reads (
        conversation bigint not null references courier_conversations (id),
        reader_template text not null,
        reader_index bigint not null,
        through bigint not null,
        primary key (conversation, reader_template, reader_index)
    )",
];

/// Make the tables where absent.
pub async fn ensure(pool: &PgPool) -> Result<(), sqlx::Error> {
    for statement in STATEMENTS {
        sqlx::query(statement).execute(pool).await?;
    }
    Ok(())
}
