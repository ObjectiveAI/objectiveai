//! `/tool/read-resource`: one resource of the container's server, read — the shared
//! [`mcp::read_resource`](crate::shared::mcp::read_resource) re-exported,
//! [`request::Request`] the params the first message carries and
//! [`response::Frame`] the one message that answers, and, behind the
//! `server` feature, the executor that asks.

pub use crate::shared::mcp::read_resource::*;

#[cfg(feature = "server")]
pub mod execute;
