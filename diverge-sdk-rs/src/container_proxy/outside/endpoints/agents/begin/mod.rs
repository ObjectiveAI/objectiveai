//! Beginning the server's work on an agent container.
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
//! container's server has taken them — carrying what the server
//! declared in return, the dependencies for the provider to have
//! deployed and the secrets it takes for the provider to have
//! resolved, see [`Registered`](server::response::Registered) — and
//! then the agent's conversation there — every chunk, for as long as
//! the connection lives, each loop's end carrying the rotating
//! secrets back; the finish is the proxy ending. The secrets' values
//! come down a channel of the server's,
//! [`Secrets`](client::channel_request::Frame::Secrets), before
//! anything is delivered to the program.
//!
//! # Channels go both ways here
//!
//! The proxy opens them for everything the container asks of the world
//! outside — its database connections, its daemon connection, its
//! tool calls outward. The server opens them for the
//! image's DIVERGE.md, for the family's own exchanges — its queue — and
//! for its half of each database connection. Same scope, opposite
//! directions, and neither side's channel numbers mean anything to the
//! other.

pub mod client;
pub mod server;
