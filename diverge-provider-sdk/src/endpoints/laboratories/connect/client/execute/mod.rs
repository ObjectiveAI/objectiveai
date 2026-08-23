//! Performing the exchange, rather than describing it.
//!
//! [`execute`] joins a laboratory and hands back two things:
//! [`ExecuteStream`], the container's filesystem, and
//! [`ExecuteHandle`], everything a connector can say back.
//! [`ReadStream`] and [`McpNotificationsStream`] are what two of its
//! eight asks answer with.
//!
//! Its own files are flattened into it, so everything is named
//! through this module and not through the file it lives in.

mod execute;
mod execute_handle;
mod execute_stream;
mod mcp_notifications_stream;
mod read_stream;

pub use execute::*;
pub use execute_handle::*;
pub use execute_stream::*;
pub use mcp_notifications_stream::*;
pub use read_stream::*;
