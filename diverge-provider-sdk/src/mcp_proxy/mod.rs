//! The wire between a provider and the MCP proxy inside a container.
//!
//! An agentic loop container runs an MCP proxy: to the agent beside
//! it, a fully compliant MCP server on the container's loopback; to
//! the provider outside, a WebSocket listener. The agent's MCP
//! exchanges leave the container on that socket, and this module is
//! the frames they ride.
//!
//! Every message is one WebSocket BINARY frame:
//!
//! ```text
//! [type: u8][channel: u8][payload…]
//! ```
//!
//! Two bytes, always — see [`HEADER_LEN`]. The same arguments as the
//! main protocol's header, at the size this wire needs: no length
//! prefix because WebSocket already delimits messages, and a fixed
//! offset because a header nobody has to parse is a header nobody
//! gets wrong.
//!
//! # It is the channel discipline, in miniature
//!
//! One channel is one exchange: a request from the proxy, the
//! responses that answer it, and a finish — after which the channel
//! is dead and nothing follows on it. A stream ends at its finish
//! frame and nowhere else; a quiet channel is a channel still
//! running. A finish with no response preceding it states that the
//! exchange could not be served. Nothing is acknowledged.
//!
//! There are no scopes, because there is nothing to scope: the
//! connection serves exactly one run, and the run is the scope.
//!
//! # The proxy mints the channels
//!
//! Only the proxy opens channels, so there is one minter and nothing
//! to collide with. A channel is a `u8`: an agent's concurrent
//! exchanges are counted in the handful, and 256 live channels is
//! headroom, not a ceiling anyone approaches.
//!
//! The number is unique among the proxy's LIVE channels. Reusing one
//! whose responses have not finished makes two exchanges
//! indistinguishable, and the proxy is the only party that could have
//! prevented it. Reuse after a finish is fine, because nothing
//! remembers.
//!
//! # One connection, and the proxy waits for it
//!
//! The proxy accepts exactly one WebSocket connection at a time. A
//! second connection while one is live is refused. A connection dying
//! is not an ending: the channels open on it are dead — their
//! exchanges were not served — and the proxy waits for the next
//! connection, blocking new exchanges until it arrives.
//!
//! # The payloads are the endpoint's own
//!
//! Nothing here is discriminated by the envelope. A channel request's
//! payload is the exchange encoding the endpoint layer already
//! defines — one exchange tag byte, then the params, exactly as
//! [`channel_request::Frame`](crate::endpoints::agentic_loop::run::server::channel_request)
//! encodes it — and a channel response's payload is that exchange's
//! response encoding, result or error, as defined in
//! [`shared::mcp`](crate::shared::mcp). This module splits a header
//! and names a frame kind; what a payload MEANS belongs to the
//! protocol carrying it.
//!
//! # Types
//!
//! | type | proxy | provider |
//! |------|-------|----------|
//! | 0    | channel request | — |
//! | 1    | — | channel response |
//! | 2    | — | channel response finish |
//!
//! A number means one thing in one direction, and each direction
//! sends only its own: the proxy asks, the provider answers. A type
//! alone determines what a frame is.

pub mod provider;
pub mod proxy;

mod error;

pub use error::{FrameError, HEADER_LEN};
use error::split_header;
