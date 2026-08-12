//! The client side of the agentic loop: what a client sends.
//!
//! [`request`] is the one request it opens a scope with. [`response`]
//! is everything it sends back on the channels the server opens inside
//! that scope.
//!
//! What comes back from the SERVER — the chunks of the loop itself —
//! is the server's to send, and lives with the rest of what a server
//! sends.

pub mod request;
pub mod response;
