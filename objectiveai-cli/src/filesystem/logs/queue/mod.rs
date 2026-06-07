//! Typed queue items returned by
//! [`crate::filesystem::Client::read_new_from_queue`].
//!
//! These re-export the SDK wire types — the on-disk persistence
//! layer and the bare-naked `agents read` Response use the same
//! shapes (one [`RequestMessageKind`] variant per row), so there's
//! no benefit to a CLI-local duplicate.
//!
//! [`RequestMessageKind`]: objectiveai_sdk::cli::command::agents::instances::read::subscribe::RequestMessageKind

pub use objectiveai_sdk::cli::command::agents::instances::read::all::{
    ResponseContent as Content,
    ResponseQueueItem as QueueItem,
    ResponseQueueMessage as QueueMessage,
};
