//! The client side of the agentic loop: what a client sends, and what
//! it gets back.

pub mod request;
pub mod response;

mod body_frame;
mod mcp;

pub use body_frame::*;
pub use mcp::*;
