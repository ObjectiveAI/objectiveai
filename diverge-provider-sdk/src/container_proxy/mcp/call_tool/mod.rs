//! The `/mcp/call-tool` path: one MCP exchange, answered once — one response, then the finish.
//!
//! An exchange path (see [the module](super::super)): the container
//! opens a channel with one [`request::Frame`], the server answers
//! with [`response::Frame`]s, typed with the exchange's own shapes
//! from [`shared::mcp::call_tool`](crate::shared::mcp::call_tool).
//!
//! # Types
//!
//! | type | server |
//! |------|--------|
//! | 0    | channel response |
//! | 1    | channel response finish |

pub mod request;
pub mod response;

mod error;

pub use error::{FrameError, HEADER_LEN};
use error::split_header;
