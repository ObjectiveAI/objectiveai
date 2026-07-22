//! Postgres backing for the `tasks` feature — durable scheduled
//! commands. The live half (the resident scheduler that claims and
//! fires due rows) lives in [`crate::command::tasks::scheduler`];
//! everything durable lives here, mirroring [`super::channels`].
//!
//! Schema: the `objectiveai.tasks` table in the root `db/schema.sql`
//! (applied by [`super::init`]).

mod row;
pub use row::*;
mod write;
pub use write::*;
mod read;
pub use read::*;
