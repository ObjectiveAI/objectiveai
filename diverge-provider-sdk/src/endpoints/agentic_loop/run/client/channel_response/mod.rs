//! The answers a client sends on the channels a server opened.
//!
//! [`mcp`] is the only one, and its frame is an ALIAS of
//! [`http::response::Frame`](crate::shared::http::response::Frame) —
//! the head-then-body split is a fact about HTTP rather than about
//! this channel, so it is defined once where every tunneled exchange
//! shares it, and named here where a reader looks for it.
//!
//! The module stays in the path rather than being re-exported upward.
//! It is what would tell two types called `Frame` apart, and a second
//! kind of channel is the sort of thing that gets added.
//!
//! Note what is NOT here. The chunks of the loop itself are a response
//! too, but the SERVER sends those, so they live in
//! [`server::response`](crate::endpoints::agentic_loop::run::server::response). This
//! module is the other direction — a client answering what it was
//! asked for.

pub mod mcp;
