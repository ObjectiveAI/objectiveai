//! Postgres-backed state for the CLI.
//!
//! Replaces the legacy `filesystem::db` SQLite tree. One sqlx `PgPool`
//! over the postgres configured via `config db` (locally provisioned
//! by `objectiveai db spawn`, or any remote instance); every tier
//! ([`tags`], [`message_queue`], [`tasks`], [`logs`]) takes `&Pool` and
//! runs natively async.

mod error;
pub use error::*;

mod pool;
pub use pool::*;

mod init;
pub use init::*;

mod lazy;
pub use lazy::*;

pub mod agent_continuations;
pub mod instances;
pub mod logs;
pub mod query;
pub mod tags;
pub mod tag_groups;
pub mod message_queue;
pub mod tasks;
