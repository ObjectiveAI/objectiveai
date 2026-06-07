//! Embedded-postgres-backed state for the CLI.
//!
//! Replaces the legacy `filesystem::db` SQLite tree. One sqlx `PgPool`
//! over the postmaster spun up by [`crate::postgres::bootstrap`]; every
//! tier ([`tags`], [`prompts`], [`messages`], [`tasks`], [`schema`])
//! takes `&Pool` and runs natively async.

mod error;
pub use error::*;

mod pool;
pub use pool::*;

mod init;
pub use init::*;

pub mod pending;
pub mod schema;
pub mod tags;
pub mod prompts;
pub mod messages;
pub mod tasks;
