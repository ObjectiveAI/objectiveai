//! The client side of the agentic loop: what a client sends, and what
//! it gets back.

pub mod request;
pub mod response;

mod mcp;
mod mcp_response_frame;
mod postgres_response_frame;

pub use mcp::*;
pub use mcp_response_frame::*;
pub use postgres_response_frame::*;
