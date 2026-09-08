//! The answers a provider sends on the channels a caller opened.
//!
//! [`filetree`] is the container's tree, [`read`] a file's bytes,
//! [`write_path`] whether one landed, [`postgres`] what the container
//! wrote on a database connection. [`agentic_loop`] is the loop, chunk
//! by chunk, and [`agent_schema`] what its agent may be — the family's
//! own, each an alias of the shape
//! [`shared::containers`](crate::shared::containers) defines.

pub mod agent_schema;
pub mod agentic_loop;
pub mod filetree;
pub mod postgres;
pub mod read;
pub mod write_path;
