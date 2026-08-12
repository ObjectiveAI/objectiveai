//! The client side of the agentic loop: what a client sends, and what
//! it gets back.

pub mod request;
pub mod response;

mod response_frame;
mod mcp;

pub use response_frame::*;
pub use mcp::*;
