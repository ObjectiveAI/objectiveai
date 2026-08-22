//! How an agent inside a container asks for a tool call.
//!
//! An [`agentic_loop`](crate::endpoints::agentic_loop::run) runs its
//! agent in a container beside the provider, and the MCP servers that
//! agent calls live with the CALLER. So a tool call has to travel out
//! of the container, across the provider, and down a channel to the
//! caller's [`McpProxy`](crate::client::mcp_proxy::McpProxy) — and the
//! first of those three legs is this.
//!
//! # The container listens and the provider dials
//!
//! Which is the shape, and it is forced. A provider cannot put a
//! listening socket inside somebody else's container, so anything a
//! container wants dialled FOR it has to be something the container is
//! listening on. The same inversion a plugin's
//! [`postgres`](crate::endpoints::mcp_plugin::run::client::request::Frame::postgres_port)
//! and
//! [`command`](crate::endpoints::mcp_plugin::run::client::request::Frame::command_port)
//! conduits make, for the same reason.
//!
//! So the provider declares the port in
//! [`Deployment::ports`](super::deployment::Deployment::ports), dials
//! it with
//! [`Container::connect`](super::container::Container::connect), and
//! then LISTENS on a connection it opened. Nothing new is needed
//! anywhere: no address a container has to be told, no endpoint a
//! provider has to stand up, and nothing in this crate serving HTTP.
//!
//! # One frame, both directions
//!
//! ```text
//! [length: u32 BE][exchange: u32 BE][payload: length bytes]
//! ```
//!
//! | direction | payload |
//! |-----------|---------|
//! | up, container to provider | an encoded [`http::request::Request`] |
//! | down, provider to container | an encoded [`http::response::Frame`] |
//! | down, `length` of `0` | the exchange is over |
//!
//! The payloads are the crate's own types and are documented where they
//! live. Going up it is the same [`Request`] the provider then relays
//! verbatim in a
//! [`channel_request::Frame`](crate::endpoints::agentic_loop::run::server::channel_request::Frame);
//! coming down it is the same [`Frame`] the caller answered with. So
//! nothing is translated in the middle, and an image author reads one
//! set of documentation rather than two.
//!
//! A zero length is unambiguous because a real payload is never empty:
//! both types begin with something, and a
//! [`Frame`] begins with a tag byte.
//!
//! # Why an exchange number
//!
//! Because an agent makes several tool calls at once and their answers
//! come back interleaved. The number is what tells them apart, and it
//! is the container's to mint because the container is the end that
//! starts an exchange.
//!
//! Running an HTTP server codec over this pipe was the alternative and
//! is worse for exactly this reason: HTTP/1.1 on one connection is
//! serial, so parallel tool calls would queue behind each other for no
//! reason but the framing.
//!
//! # What it does not do
//!
//! **Say what an exchange means.** The provider relays and never
//! parses: it does not read JSON-RPC, does not track MCP sessions, and
//! never looks at the `Mcp-Session-Id` that ties a caller's exchanges
//! together. What server a call reaches is settled at the far end, by
//! the caller's proxy, which is why neither the
//! [`request`](crate::endpoints::agentic_loop::run::client::request)
//! nor the relayed frame names one.
//!
//! **Cancel.** There is no way to abandon an exchange in flight from
//! either end short of dropping the whole conduit. Nothing needs it
//! yet.
//!
//! [`Frame`]: crate::shared::http::response::Frame
//! [`Request`]: crate::shared::http::request::Request
//! [`http::request::Request`]: crate::shared::http::request::Request
//! [`http::response::Frame`]: crate::shared::http::response::Frame

mod message;

pub use message::*;
