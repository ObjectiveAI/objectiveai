//! The messages, in the tool's database.
//!
//! One table: every message between the person and an agent, keyed by
//! the agent's template and index. A person's message is unread until
//! the agent reads it; an agent's message is never marked.

use chrono::{DateTime, Utc};
use diverge_sdk::container_proxy::inside::Client;
use serde::Serialize;
use sqlx::postgres::{PgPoolOptions, PgRow};
use sqlx::{FromRow, PgPool, Row as _};
use tokio::sync::OnceCell;

use crate::caller::Participant;
use crate::failure::Failure;

/// Every statement that makes the table, in order; each `if not
/// exists`, so ensuring it again is nothing.
const STATEMENTS: [&str; 2] = [
    "create table if not exists chat_messages (
        id bigserial primary key,
        template text not null,
        index bigint not null,
        author text not null check (author in ('user', 'agent')),
        name text,
        issuer text,
        subject text,
        body text not null,
        read boolean not null default false,
        created timestamptz not null default now()
    )",
    "create index if not exists chat_messages_by_agent on chat_messages (template, index, id)",
];

/// The columns a message is read from, in the order [`Message`] reads.
const COLUMNS: &str = "id, template, index, author, name, issuer, subject, body, read, created";

/// One message, as the page and the agent read it.
///
/// `author` is `user` or `agent`. A person's message carries the
/// account that wrote it — `issuer` and `subject`, from the daemon's
/// stamp; an agent's carries the agent's `name` as it was when it
/// wrote, if it had one.
#[derive(Debug, Clone, Serialize)]
pub struct Message {
    /// The message's id, increasing in the order messages are written.
    pub id: i64,
    /// The agent the conversation is with: its template.
    pub template: String,
    /// The agent the conversation is with: its index.
    pub index: i64,
    /// `user` or `agent`.
    pub author: String,
    /// The agent's name, on an agent's message.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// The account's issuer, on a person's message.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub issuer: Option<String>,
    /// The account's subject, on a person's message.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subject: Option<String>,
    /// What was written.
    pub body: String,
    /// Whether the agent has read it; an agent's own message is never.
    pub read: bool,
    /// When it was written.
    pub created: DateTime<Utc>,
}

impl FromRow<'_, PgRow> for Message {
    fn from_row(row: &PgRow) -> Result<Self, sqlx::Error> {
        Ok(Message {
            id: row.try_get("id")?,
            template: row.try_get("template")?,
            index: row.try_get("index")?,
            author: row.try_get("author")?,
            name: row.try_get("name")?,
            issuer: row.try_get("issuer")?,
            subject: row.try_get("subject")?,
            body: row.try_get("body")?,
            read: row.try_get("read")?,
            created: row.try_get("created")?,
        })
    }
}

/// The pool, made and the table ensured on first use. A failure leaves
/// nothing behind, so the next use tries again.
#[derive(Default)]
pub struct Store {
    pool: OnceCell<PgPool>,
}

impl Store {
    /// The pool, through the proxy's loopback pgwire.
    pub async fn pool(&self, client: &Client) -> Result<&PgPool, Failure> {
        self.pool
            .get_or_try_init(|| async {
                let pool = PgPoolOptions::new().max_connections(4).connect(&client.postgres_url()).await?;
                for statement in STATEMENTS {
                    sqlx::query(statement).execute(&pool).await?;
                }
                Ok::<_, Failure>(pool)
            })
            .await
    }
}

/// Keep a person's message to `agent`, written by the account
/// `issuer`/`subject`.
pub async fn user(pool: &PgPool, agent: &Participant, issuer: &str, subject: &str, body: &str) -> Result<Message, Failure> {
    let statement = format!("insert into chat_messages (template, index, author, issuer, subject, body) values ($1, $2, 'user', $3, $4, $5) returning {COLUMNS}");
    Ok(sqlx::query_as(&statement).bind(&agent.template).bind(agent.index).bind(issuer).bind(subject).bind(body).fetch_one(pool).await?)
}

/// Keep `agent`'s message.
pub async fn agent(pool: &PgPool, agent: &Participant, body: &str) -> Result<Message, Failure> {
    let statement = format!("insert into chat_messages (template, index, author, name, body) values ($1, $2, 'agent', $3, $4) returning {COLUMNS}");
    Ok(sqlx::query_as(&statement).bind(&agent.template).bind(agent.index).bind(&agent.name).bind(body).fetch_one(pool).await?)
}

/// Every message with `agent`, either way, oldest first.
pub async fn history(pool: &PgPool, template: &str, index: i64) -> Result<Vec<Message>, Failure> {
    let statement = format!("select {COLUMNS} from chat_messages where template = $1 and index = $2 order by id");
    Ok(sqlx::query_as(&statement).bind(template).bind(index).fetch_all(pool).await?)
}

/// The person's messages to `agent`, oldest first: the unread ones, or
/// every one with `include_read`. Every unread one among them is marked
/// read, in the same transaction, as it is handed back as it was.
pub async fn read(pool: &PgPool, agent: &Participant, include_read: bool) -> Result<Vec<Message>, Failure> {
    let mut transaction = pool.begin().await?;
    let filter = if include_read { "" } else { " and not read" };
    let statement = format!("select {COLUMNS} from chat_messages where template = $1 and index = $2 and author = 'user'{filter} order by id for update");
    let messages: Vec<Message> = sqlx::query_as(&statement).bind(&agent.template).bind(agent.index).fetch_all(&mut *transaction).await?;
    sqlx::query("update chat_messages set read = true where template = $1 and index = $2 and author = 'user' and not read")
        .bind(&agent.template)
        .bind(agent.index)
        .execute(&mut *transaction)
        .await?;
    transaction.commit().await?;
    Ok(messages)
}
