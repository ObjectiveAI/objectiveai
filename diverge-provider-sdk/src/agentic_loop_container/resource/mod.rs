//! Delivering a resource's bytes into the container.
//!
//! The second half of the exchange the response stream's
//! [`FetchResource`](super::response::FetchResource) event opens:
//! the container asked for an identity's bytes, the server obtained
//! them (from the client over the wire's own FetchResource
//! exchange, or from its own store), and these routes are how the
//! delivery lands — on the loop port, one route per thing that can
//! happen to it:
//!
//! - `POST /resource/{identity}` — one chunk, the body the bytes
//!   verbatim ([`Request`]); one POST per chunk.
//! - `POST /resource/{identity}/complete` — every chunk is in
//!   ([`complete`]).
//! - `POST /resource/{identity}/error` — no more are coming, ever
//!   ([`error`]) — the delivery's other ending.
//!
//! No enum, no tag byte: the ROUTE says which, the way the verb
//! routes beside it already work, and each body is exactly its own
//! payload — bytes for the chunk (binary, because base64 is a third
//! more wire for nothing), JSON for the endings, which are
//! documents.
//!
//! # The identity is the path
//!
//! `{identity}` is one path segment carrying the resource's
//! size-bearing identity verbatim — the FILE grammar,
//! `f1:<size>:<base64url sha256 of the bytes>`, every character of
//! which is legal in a path segment as-is, so nothing needs
//! percent-encoding (a framework that decodes anyway changes
//! nothing). Stated HERE because it can only be stated: a type can
//! carry a body, but a route's path is convention, and this
//! paragraph is that convention's one home.
//!
//! # Chunks append; a settlement ends it
//!
//! Chunks of one identity — one path — arrive in POST order: the
//! server posts one at a time, each answered before the next, and
//! the receiver appends, never measures: the sender's chunking (at
//! most
//! [`CHUNK_SIZE`](crate::CHUNK_SIZE)
//! per POST) is the sender's business. The completion or the error
//! settles the identity — exactly one, and nothing lands after it.
//! A lone completion with no chunks before it delivers the empty
//! resource. Whether what arrived is RIGHT is the identity's own
//! promise: the size and the hash it carries are exactly what a
//! wrong delivery fails.
//!
//! # Failures are HTTP's own
//!
//! Per the surface's contract: a delivery for an identity the
//! container never asked for, or one already settled, is a `409`;
//! an endings body that does not parse is a `400`; and the 2xx says
//! only that the POST was taken — [`Response`], one shape for all
//! three routes. (A chunk cannot be malformed: any bytes are a
//! chunk.)

mod request;
mod response;

pub use request::*;
pub use response::*;

pub mod complete;
pub mod error;
