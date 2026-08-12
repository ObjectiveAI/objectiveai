//! The server side of the agentic loop: what a provider sends.
//!
//! Its answer to the client's request, on channel `0`, and the
//! requests it opens channels of its own to make. Both happen inside
//! the scope the client's request opened — a server never initiates
//! one.

mod response_frame;
mod mcp;
mod request_frame;

pub use response_frame::*;
pub use mcp::*;
pub use request_frame::*;
