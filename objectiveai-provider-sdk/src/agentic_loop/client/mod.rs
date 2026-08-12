//! The client side of the agentic loop: what a client sends.
//!
//! Its one request, and its answers on the channels the server opens.
//! What comes BACK from the server — the chunks of the loop itself —
//! is the server's to send, and lives with the rest of what a server
//! sends.
//!
//! [`mcp`] and [`postgres`] each hold the answers for one kind of
//! channel, and each calls its own type `Frame`. Nothing re-exports
//! them upward: the module is what tells the two apart, so it has to
//! stay in the path.

pub mod mcp;
pub mod postgres;
pub mod request;
