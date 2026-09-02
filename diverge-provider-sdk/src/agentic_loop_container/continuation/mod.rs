//! Delivering the continuation into the container.
//!
//! The second half of the exchange the response stream's
//! [`FetchContinuation`](super::response::FetchContinuation) event
//! opens: the container asked for the continuation it resumes from,
//! the server obtained it (from the client over the wire's own
//! fetch_continuation exchange), and these routes are how it lands —
//! on the loop port, one route per thing that can happen to it,
//! exactly as a [`resource`](super::resource) lands, minus the path
//! segment, because there is nothing to name:
//!
//! - `POST /continuation` — one chunk, the body the bytes verbatim
//!   ([`Request`]); one POST per chunk.
//! - `POST /continuation/complete` — every chunk is in
//!   ([`complete`]).
//! - `POST /continuation/error` — no more are coming, ever
//!   ([`error`]) — the delivery's other ending.
//!
//! # Chunks are kept, in order; a settlement ends it
//!
//! Chunks arrive in POST order — the server posts one at a time,
//! each answered before the next — and each POST is exactly one of
//! the chunks the earlier run's closer sent, kept apart all the way
//! from that container to the caller and back. The store keeps them
//! apart too: a container reads its continuation as the sequence it
//! minted, boundaries intact, and may have put meaning in them. The
//! completion or the error settles the delivery — exactly one, and
//! nothing lands after it.
//!
//! # A lone completion is a fresh start
//!
//! The one place this differs from a resource. A completion with no
//! chunks before it is the wire's empty finish delivered: the caller
//! holds nothing to resume from, and the run begins the conversation
//! here — the ordinary first run, not a refusal. And no identity
//! backs these bytes: whether a delivery is whole is the container's
//! own to judge, by its own format, when it opens them.
//!
//! # Failures are HTTP's own
//!
//! Per the surface's contract: a delivery after the settlement is a
//! `409`; an endings body that does not parse is a `400`; and the
//! 2xx says only that the POST was taken — [`Response`], one shape
//! for all three routes. (A chunk cannot be malformed: any bytes are
//! a chunk.)

mod request;
mod response;

pub use request::*;
pub use response::*;

pub mod complete;
pub mod error;
