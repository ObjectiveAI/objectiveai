//! Typed queue items returned by
//! [`crate::filesystem::Client::read_new_from_queue`].
//!
//! Mirrors the schema spelled out in `WORK.md`: each enum variant
//! corresponds to one [`MessageKind`] row, with the per-row file(s)
//! read and flattened to bare [`Id`] paths (relative to the logs
//! directory).
//!
//! [`MessageKind`]: crate::filesystem::db::schema::MessageKind

mod content;
mod id;
mod queue_item;
mod queue_message;

pub use content::*;
pub use id::*;
pub use queue_item::*;
pub use queue_message::*;
