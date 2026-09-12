//! `/tool/list-tools`: what tools the container's server offers — the shared
//! [`mcp::list_tools`](crate::shared::mcp::list_tools) re-exported,
//! [`request::Request`] the params the first message carries and
//! [`response::Frame`] the one message that answers, and, behind the
//! `server` feature, the executor that asks.

pub use crate::shared::mcp::list_tools::*;

#[cfg(feature = "server")]
pub mod execute;
