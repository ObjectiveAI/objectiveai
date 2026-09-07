//! An MCP server in a container.
//!
//! The image runs an MCP server, and the caller speaks to it over the
//! five exchanges [`shared::mcp`](crate::shared::mcp) defines, each on
//! a channel of its own. Nothing else distinguishes it from an agent
//! container: the same request makes it, the same channels read and
//! write its files and watch its tree, and it asks the same things of
//! the caller — a database, a command, its vault, tools of its own.
//!
//! [`run`] owns the container; [`connect`] joins one.

pub mod connect;
pub mod run;
