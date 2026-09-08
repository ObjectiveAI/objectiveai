//! The answers a provider sends on the channels a caller opened.
//!
//! [`filetree`] is the container's tree, [`read`] a file's bytes,
//! [`write_path`] whether one landed, [`postgres`] what the container
//! wrote on a database connection. [`run_loop`] is the loop, chunk
//! by chunk, [`agent_schema`] what its agent may be, and [`enqueue`]
//! and [`dequeue`] the fates of what a caller put in the running
//! loop's queue — the family's own, each an alias of the shape
//! [`shared::containers`](crate::shared::containers) defines.

pub mod agent_schema;
pub mod dequeue;
pub mod enqueue;
pub mod filetree;
pub mod postgres;
pub mod read;
pub mod run_loop;
pub mod write_path;
