//! The answers a provider sends on the channels a caller opened.
//!
//! [`filetree`] is the container's tree, [`read`] a file's bytes,
//! [`write_path`] whether one landed, [`postgres`] what the container
//! wrote on a database connection. [`agent_schema`] is what its
//! agent may be, and [`enqueue`] and [`dequeue`] the fates of what a
//! caller sent the agent — the family's own, each an alias of the
//! shape [`shared::containers`](crate::shared::containers) defines.
//! What the agent says is not here: it rides the scope's own main
//! stream, [`response`](super::response).

pub mod agent_schema;
pub mod dequeue;
pub mod enqueue;
pub mod filetree;
pub mod postgres;
pub mod read;
pub mod write_path;
