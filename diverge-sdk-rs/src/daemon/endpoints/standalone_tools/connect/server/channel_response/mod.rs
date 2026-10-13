//! What the daemon answers each channel of a connect with, one module
//! per channel the client opens.
//!
//! The five `mcp_*` are what the tool's server answered, each the
//! exchange's own frame of [`shared::mcp`](crate::shared::mcp), aliased
//! as the provider protocol aliases the same five: the four that
//! answer once are one channel response then the finish, the
//! notifications one per notification until the scope ends. The
//! disconnect is answered by nothing: the scope's finish is its answer.

pub mod mcp_call_tool;
pub mod mcp_list_resources;
pub mod mcp_list_tools;
pub mod mcp_notifications;
pub mod mcp_read_resource;
