//! Beginning the server's work on a tool container.
//!
//! Split by who SENDS: [`client`] is the server's traffic, [`server`]
//! the proxy's.
//!
//! The scope a begin opens is the CONNECTION's life. It is the server's
//! first act, once, and it carries the arguments — the image's, a JSON
//! value, handed over here and never again — and the template's id,
//! and nothing else: what it opens is the place channels go, and a
//! container that holds its arguments. The proxy answers
//! [`Begun`](server::response::Frame::Begun) on channel `0` once the
//! container's server has taken them — carrying the secrets the
//! server declared it takes, for the provider to have resolved, see
//! [`Registered`](server::response::Registered) — and then, for as
//! long as the connection lives, its word whenever the container
//! goes active and inactive, the latter carrying the rotating secrets
//! back; the finish is the proxy ending. The secrets' values come
//! down a channel of the server's,
//! [`Secrets`](client::channel_request::Frame::Secrets), before
//! anything is delivered to the program.
//!
//! # Channels go both ways here
//!
//! The proxy opens them for everything the container asks of the world
//! outside — its database connections, its daemon connection, its
//! tool calls outward. The server opens them for the
//! image's DIVERGE.md, for the family's own exchange — the five MCP
//! exchanges into the container's server — and for its half of each
//! database connection. Same scope, opposite directions, and neither
//! side's channel numbers mean anything to the other.

pub mod client;
pub mod server;
