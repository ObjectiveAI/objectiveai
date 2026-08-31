//! Delivering a resource's bytes into the container.
//!
//! The second half of the exchange the response stream's
//! [`FetchResource`](super::response::FetchResource) event opens:
//! the container asked for an identity's bytes, the server obtained
//! them (from the client over the wire's own FetchResource
//! exchange, or from its own store), and this route is how they
//! arrive — `POST /resource` on the loop port, one POST per chunk,
//! then one more saying the resource is whole.
//!
//! # One route, a tag says which
//!
//! The body is BINARY, not JSON — the payload is bytes, and a JSON
//! body would mean base64 and a third more wire for nothing — so
//! the two asks share the route the way the wire's frames share a
//! channel: the body leads with one byte saying which, and the rest
//! is that ask's own. See [`Request`].
//!
//! # Chunks append; the completion says whole
//!
//! Chunks of one identity arrive in POST order — the server posts
//! one at a time, each answered before the next — and the receiver
//! appends, never measures: the sender's chunking (at most
//! [`CHUNK_SIZE`](crate::endpoints::agentic_loop::run::client::channel_response::CHUNK_SIZE)
//! per POST) is the sender's business. The completion POST is what
//! says every chunk is in; a lone completion with no chunks before
//! it delivers the empty resource. Whether what arrived is RIGHT is
//! the identity's own promise: the size and the hash it carries are
//! exactly what a wrong delivery fails.
//!
//! # Failures are HTTP's own
//!
//! Per the surface's contract: a body that does not parse is a
//! `400`, a delivery for an identity the container never asked for
//! is a `409`, and the 2xx says only that the POST was taken —
//! [`Response`], one shape for both tags.

mod request;
mod response;

pub use request::*;
pub use response::*;
