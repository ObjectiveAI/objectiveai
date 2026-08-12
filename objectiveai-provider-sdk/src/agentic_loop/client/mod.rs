//! The client side of the agentic loop: what a client sends.
//!
//! Its one request, and its answers on the channels the server opens.
//! What comes BACK from the server — the chunks of the loop itself —
//! is the server's to send, and lives with the rest of what a server
//! sends.

pub mod mcp;
pub mod request;

mod postgres_response_frame;

pub use postgres_response_frame::*;
