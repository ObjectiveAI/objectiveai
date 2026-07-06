//! Postgres-backed state for the CLI.
//!
//! Replaces the legacy `filesystem::db` SQLite tree. One sqlx `PgPool`
//! over the objectiveai-db cluster whose `postgresql://` URL is
//! published in the `db` spawn lock (or a remote postgres pointed at
//! via `db config address`), lazily initialized by
//! `Context::db_client()`; every tier ([`tags`], [`message_queue`],
//! [`logs`]) takes `&Pool` and runs natively async.

mod error;
pub use error::*;

mod pool;
pub use pool::*;

mod init;
pub use init::*;

pub mod agent_continuations;
pub mod agent_refs;
pub mod compartment;
pub mod instances;
pub mod laboratory_attachments;
pub mod logs;
pub mod query;
pub mod tags;
pub mod time;
pub mod tag_groups;
pub mod message_queue;
