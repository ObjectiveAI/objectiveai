//! Embedded-postgres-backed state for the CLI.
//!
//! Replaces the legacy `filesystem::db` SQLite tree. One sqlx `PgPool`
//! over the postmaster spun up by [`crate::postgres::bootstrap`]; every
//! tier ([`tags`], [`message_queue`], [`tasks`], [`logs`]) takes `&Pool` and
//! runs natively async.

mod error;
pub use error::*;

mod pool;
pub use pool::*;

mod init;
pub use init::*;

pub mod agent_continuations;
pub mod logs;
pub mod tags;
pub mod tag_groups;
pub mod message_queue;
pub mod tasks;
