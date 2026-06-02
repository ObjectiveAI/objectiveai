//! Typed queue items returned by
//! [`crate::filesystem::Client::read_new_from_queue`].
//!
//! Mirrors the schema spelled out in `WORK.md`: each enum variant
//! corresponds to one [`RequestMessageKind`] row, with the per-row file(s)
//! read and flattened to bare `i64` SQL row ids (into the `files`
//! table populated by [`crate::filesystem::Client::read_new_from_queue`]).
//!
//! [`RequestMessageKind`]: objectiveai_sdk::cli::command::agents::read::subscribe::RequestMessageKind

mod content;
mod queue_item;
mod queue_message;

pub use content::*;
pub use queue_item::*;
pub use queue_message::*;
