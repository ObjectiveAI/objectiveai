//! The channels a server opens on a client.
//!
//! Five, and every one is a connection the provider cannot make itself:
//! the agent runs beside it, and the MCP servers live with the client.
//! [`Frame`] is what opens one, and it is the only frame here.
//!
//! What it carries is `rmcp`'s own params, which is not here and should
//! not be. An MCP request has one shape and it is MCP's; writing it a
//! second time in a second place is how two shapes start.
//!
//! There was one channel, carrying a tunneled HTTP request, on the
//! grounds that a relay which never parsed could never be wrong about
//! what it was relaying. What that cost was an envelope neither end
//! read: a method, a path and headers that a provider fabricated and a
//! terminator discarded.

mod frame;

pub use frame::*;
