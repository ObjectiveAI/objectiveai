//! The answers a client sends on the channels a server opened.
//!
//! One module per kind of channel, each naming its own type `Frame`.
//! Nothing is re-exported upward: the module is the only thing telling
//! two types called `Frame` apart, so it has to stay in the path.
//!
//! [`mcp`] is [`http::response`](crate::http::response) under a
//! shorter name, not a copy of it — the head-then-body split is a fact
//! about HTTP rather than about this channel, so it is defined once
//! where every tunneled exchange shares it.
//!
//! Note what is NOT here. The chunks of the loop itself are a response
//! too, but the SERVER sends those, so they live in
//! [`server::response`](crate::agentic_loop::server::response). This
//! module is the other direction — a client answering what it was
//! asked for.

pub use crate::http::response as mcp;

pub mod postgres;
