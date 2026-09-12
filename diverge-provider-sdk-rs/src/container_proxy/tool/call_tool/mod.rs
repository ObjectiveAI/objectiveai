//! `/tool/call-tool`: one tool of the container's server, called — the shared
//! [`mcp::call_tool`](crate::shared::mcp::call_tool) re-exported,
//! [`request::Request`] the params the first message carries and
//! [`response::Frame`] the one message that answers, and, behind the
//! `server` feature, the executor that asks.

pub use crate::shared::mcp::call_tool::*;

#[cfg(feature = "server")]
pub mod execute;
