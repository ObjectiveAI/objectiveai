//! Postgres backing for the `channels` feature — durable channel
//! records + their append-only message logs. The live coordination
//! (SSE connections, offer/accept, publish-blocking) lives in
//! [`crate::http::channel_routes`]; everything durable lives here,
//! mirroring [`super::logs`].
//!
//! Schema: the `objectiveai.channels` + `channel_messages` tables in the root `db/schema.sql` (applied by [`super::init`]).

mod row;
pub use row::*;
mod write;
pub use write::*;
mod read;
pub use read::*;
mod read_id;
pub use read_id::*;
mod listen;
pub use listen::*;
