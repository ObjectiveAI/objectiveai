//! The answers a provider sends on the channels a caller opened.
//!
//! [`filetree`] is the container's tree, [`read`] a file's bytes,
//! [`write_path`] whether one landed, [`transfer`] whether one landed
//! in another container, [`fuse_mount`] whether one more mount asked
//! for while the container runs is made, [`postgres`] what the
//! container wrote on a database connection, [`info`] what its
//! arguments may be — the image's DIVERGE.md. The other five are what the
//! container's MCP server said, one per exchange, and each is an alias
//! of the shape [`shared::mcp`](crate::shared::mcp) defines — what a
//! channel carries is what MCP says it carries, whichever direction it
//! runs.

pub mod filetree;
pub mod fuse_mount;
pub mod info;
pub mod mcp_call_tool;
pub mod mcp_list_resources;
pub mod mcp_list_tools;
pub mod mcp_notifications;
pub mod mcp_read_resource;
pub mod daemon;
pub mod postgres;
pub mod read;
pub mod transfer;
pub mod write_path;
