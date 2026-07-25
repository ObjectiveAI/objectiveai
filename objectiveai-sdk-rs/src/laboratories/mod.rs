//! Laboratories: completion-wide client-side MCP servers.
//!
//! A [`Laboratory`] attached to an agent completion is dialed by the proxy
//! as a client-side MCP upstream across *every* agent in the completion,
//! including fallbacks. Each laboratory is identified by an opaque `id`;
//! the proxy mirrors it as the URL `client://laboratory/{id}` and the CLI conduit
//! routes it via the `id`-keyed [`crate::client_objectiveai_mcp::McpKind`]
//! variant.
//!
//! Lives in a folder (rather than a flat `laboratories.rs`) so the
//! json-schema coverage test derives the `laboratories.` title prefix from
//! the directory — matching these types' `#[schemars(rename = "laboratories.…")]`.

#[cfg(feature = "laboratory-daemon")]
pub mod daemon;

mod composite;
mod container;
pub mod filetree;
mod image;
mod laboratories;

pub use composite::*;
pub use container::*;
pub use image::*;
pub use laboratories::*;
