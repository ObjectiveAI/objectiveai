//! The MCP exchange: one request out, one response back.
//!
//! What a provider's agent asks of a client's MCP proxy, and what
//! comes back. Described here ONCE, as the exchange it is — not as
//! whatever a wire happens to make of it.
//!
//! # Not a frame
//!
//! Nothing here encodes, decodes, or knows what a channel is. That
//! separation is the point, because a frame and an exchange disagree
//! about shape in one place: a [`Response`](response::Response) is a
//! status, headers and a body, whereas the response FRAME splits the
//! head from the body and sends the head first — a stream may run for
//! the length of a session, and a conduit that waited for the body to
//! finish before writing the status line would be buffering an event
//! stream it was supposed to be relaying.
//!
//! That split is a fact about the wire, not about MCP. Keeping it in
//! [`agentic_loop`](crate::agentic_loop) leaves this module free to
//! say the simple, true thing.
//!
//! # Split by direction
//!
//! [`request`] and [`response`] rather than client and server, because
//! an exchange has no sides of its own. Which end sends which is a
//! fact about the channel carrying it, and the agentic loop's own
//! modules already say so — the request travels server to client
//! there, which is the opposite of what "MCP request" suggests and
//! exactly why it is not encoded in these names.

pub mod request;
pub mod response;
