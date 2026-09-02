//! Failing the continuation's delivery into the container.
//!
//! The delivery's other ending. The server fetched the continuation
//! from the client and could not finish — the client vanished
//! mid-fetch, or refused — so the bytes will never be whole. This
//! route is how the server says so: `POST /continuation/error`, the
//! full error as the body.
//!
//! # A run cannot start from a past it does not know
//!
//! Unlike a resource, whose failure lands on one tool, a failed
//! continuation leaves the container with no honest way to begin: it
//! knows there WAS state and cannot have it. The run fails — the
//! container's business, reported on its own stream in its own
//! words; nothing here types it.
//!
//! # Failure is a settlement
//!
//! It settles the delivery exactly as the completion does: posted
//! INSTEAD of the completion, never after it, and once posted the
//! delivery takes nothing further (`409`). Whatever chunks arrived
//! before it are discarded with it.
//!
//! # Whatever "cannot" means, this carries it
//!
//! The vocabulary is the server's — the body carries the full error
//! verbatim, and the container relays it rather than interpreting
//! it.

mod request;
mod response;

pub use request::*;
pub use response::*;
