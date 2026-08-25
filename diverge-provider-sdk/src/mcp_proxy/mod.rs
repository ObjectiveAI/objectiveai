//! The wire between a provider server and the MCP proxy inside a
//! container.
//!
//! An agentic loop container runs an MCP proxy: to the agent beside
//! it, a fully compliant MCP server on the container's loopback; to
//! the server outside, a WebSocket listener. The agent's MCP
//! exchanges leave the container on that socket, and this module is
//! the frames they ride.
//!
//! Every message is one WebSocket BINARY frame. The two directions
//! frame differently, because they have different amounts to say:
//!
//! ```text
//! container → server:  [channel: u8][request…]
//! server → container:  [type: u8][channel: u8][payload…]
//! ```
//!
//! The container sends one kind of frame, so no byte says which — a
//! discriminant with nothing to discriminate is a byte not spent. The
//! server sends two, and one byte says which; see [`HEADER_LEN`].
//! Fixed headers, no length prefix: WebSocket already delimits
//! messages, and a header nobody has to parse is a header nobody gets
//! wrong.
//!
//! # It is the channel discipline, in miniature
//!
//! One channel is one exchange: a request from the container, the
//! responses that answer it, and a finish — after which the channel
//! is dead and nothing follows on it. A stream ends at its finish
//! frame and nowhere else; a quiet channel is a channel still
//! running. A finish with no response preceding it states that the
//! exchange could not be served. Nothing is acknowledged.
//!
//! There are no scopes, because there is nothing to scope: the
//! connection serves exactly one run, and the run is the scope.
//!
//! # The container mints the channels
//!
//! Only the container opens channels, so there is one minter and
//! nothing to collide with. A channel is a `u8`: an agent's
//! concurrent exchanges are counted in the handful, and 256 live
//! channels is headroom, not a ceiling anyone approaches.
//!
//! The number is unique among the container's LIVE channels. Reusing
//! one whose responses have not finished makes two exchanges
//! indistinguishable, and the container is the only party that could
//! have prevented it. Reuse after a finish is fine, because nothing
//! remembers.
//!
//! # One connection, and the proxy waits for it
//!
//! The proxy accepts exactly one WebSocket connection at a time. A
//! second connection while one is live is refused. A connection dying
//! is not an ending: the channels open on it are dead, and the proxy
//! waits for the next connection, blocking new exchanges until it
//! arrives.
//!
//! # A connection dying does not fail an exchange
//!
//! An exchange is answered when its response has arrived AND its
//! channel has finished. A channel that died before that point — its
//! connection went — left the exchange un-answered, and the container
//! asks it again on the next connection, as a fresh channel. So a
//! server may legitimately receive the same logical ask as two wire
//! exchanges, and an ask with side effects may be performed twice;
//! that is the chosen trade, because the agent inside is waiting and
//! the alternative is telling it a transport story it can do nothing
//! about. The one non-answer that is not re-asked is the deliberate
//! one: a finish with no response is the far side's statement, not an
//! accident.
//!
//! # The request is typed, and the responses are not
//!
//! A [`container::Frame`] carries its exchange as
//! [`channel_request::Frame`] — the same type a server's own channel
//! request carries on the main wire, because it is the same ask one
//! hop earlier. There is exactly one thing the frame can hold, so it
//! holds the type.
//!
//! A [`server::Frame`] response stays bytes, and not for want of a
//! type: which exchange a response answers is known only to whoever
//! opened the channel, and the payload's own tag — result or error —
//! discriminates within an exchange, not between them. The opener
//! decodes with the response type of the exchange it asked for, as
//! [`shared::mcp`](crate::shared::mcp) defines them.
//!
//! # Types
//!
//! The type space is the server's alone — the container's one frame
//! carries none.
//!
//! | type | server |
//! |------|--------|
//! | 0    | channel response |
//! | 1    | channel response finish |
//!
//! [`channel_request::Frame`]: crate::endpoints::agentic_loop::run::server::channel_request::Frame

pub mod container;
pub mod server;

mod error;

pub use error::{FrameError, HEADER_LEN};
use error::split_header;
