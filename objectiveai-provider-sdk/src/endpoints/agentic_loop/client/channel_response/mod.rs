//! The answers a client sends on the channels a server opened.
//!
//! One module per kind of channel, each naming its own type `Frame`.
//! Nothing is re-exported upward: the module is the only thing telling
//! two types called `Frame` apart, so it has to stay in the path.
//!
//! [`mcp`]'s frame is an ALIAS of
//! [`http::response::Frame`](crate::shared::http::response::Frame) — the
//! head-then-body split is a fact about HTTP rather than about this
//! channel, so it is defined once where every tunneled exchange
//! shares it, and named here where a reader looks for it.
//!
//! Note what is NOT here. The chunks of the loop itself are a response
//! too, but the SERVER sends those, so they live in
//! [`server::response`](crate::endpoints::agentic_loop::server::response). This
//! module is the other direction — a client answering what it was
//! asked for.

pub mod mcp;
pub mod postgres;
