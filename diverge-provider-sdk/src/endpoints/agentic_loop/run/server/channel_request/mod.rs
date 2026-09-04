//! The channels a server opens on a client.
//!
//! Ten, and every one is a reach the provider cannot make itself:
//! the agent runs beside it, and the MCP servers — and the store the
//! mounted content, resources and continuation live in, and the
//! database — live with the client. [`Frame`] is what opens one, and
//! it is the only frame here. [`fetch_file`], [`fetch_directory`],
//! [`fetch_resource`] and [`fetch_continuation`] are the four
//! fetches, this endpoint's and nobody else's. [`Postgres`] is the
//! tenth: HALF of a database connection — the caller opens the other
//! half — and that file is where the reason lives.
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
mod postgres;

pub use frame::*;
pub use postgres::*;

pub mod fetch_continuation;
pub mod fetch_directory;
pub mod fetch_file;
pub mod fetch_resource;
