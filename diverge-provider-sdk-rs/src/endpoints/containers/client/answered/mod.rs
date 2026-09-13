//! What one client-opened channel's frames decode to.
//!
//! [`Answered`] is the trait; the rest are its implementors for the
//! answers every scope shares — the shared `postgres`,
//! `agent_schema`, `enqueue`, `dequeue` and the five MCP frames. The
//! three answers each family envelopes in a type of its own —
//! `filetree`, `read`, `write_path` — have their implementors beside
//! that family's `execute`.

mod agent_schema;
mod answered;
mod dequeue;
mod enqueue;
mod mcp_call_tool;
mod mcp_list_resources;
mod mcp_list_tools;
mod mcp_notifications;
mod mcp_read_resource;
mod postgres;

pub use agent_schema::*;
pub use answered::*;
pub use dequeue::*;
pub use enqueue::*;
pub use mcp_call_tool::*;
pub use mcp_list_resources::*;
pub use mcp_list_tools::*;
pub use mcp_notifications::*;
pub use mcp_read_resource::*;
pub use postgres::*;
