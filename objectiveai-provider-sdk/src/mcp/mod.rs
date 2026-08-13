//! MCP: the exchange, and how it is framed.
//!
//! What a provider's agent asks of a client's MCP proxy, what comes
//! back, and the shape both take on a wire. Described here ONCE, and
//! shared by everything that carries MCP — the agentic loop today,
//! whatever else later.
//!
//! # Why the framing lives here too
//!
//! Because the framing is the interesting part, and it is the same
//! wherever MCP is carried.
//!
//! A [`request`] is whole: one complete JSON document of known length,
//! or nothing at all. A [`response`] is not — a `POST` may be answered
//! with an event stream held open while the far server works, and a
//! `GET` with one held open for the length of the SESSION. So a
//! response is split, head first, and a conduit writes the status line
//! onto the agent's socket the moment the head lands rather than
//! buffering a stream it was meant to relay.
//!
//! That asymmetry is a fact about MCP, not about any one channel. A
//! second scope carrying MCP would need exactly the same split, and
//! deriving it twice would be two chances to derive it differently.
//!
//! # Split by direction
//!
//! [`request`] and [`response`] rather than client and server, because
//! an exchange has no sides of its own. Which end sends which is a
//! fact about the channel carrying it, and the agentic loop is the
//! proof: an MCP request travels SERVER to client there, the opposite
//! of what "MCP request" suggests, and naming these for sides would
//! have made that read as a bug.

pub mod request;
pub mod response;
