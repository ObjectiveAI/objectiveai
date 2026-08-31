//! Failing a resource's delivery into the container.
//!
//! The delivery's other ending. A client disconnect does not cancel
//! the container — nothing does but the run ending — so a resource
//! the server can never deliver would otherwise leave a fetch
//! waiting on a seal that will never come. This route is how the
//! server says so instead: `POST /resource/{identity}/error`, the
//! full error as the body, and the container's waiting fetch
//! completes with the error where the bytes would have been.
//!
//! # Failure is a settlement
//!
//! It settles the identity exactly as the completion does: posted
//! INSTEAD of the completion, never after it, and once posted the
//! identity takes nothing further — no chunks, no completion, no
//! second error (`409`, like anything after a completion). What
//! chunks arrived before it are discarded with it: a failed
//! resource has no partial to salvage, because a partial fails the
//! identity's own size and hash anyway.
//!
//! # Whatever "cannot" means, this carries it
//!
//! The client disconnected mid-fetch; the client answered the
//! wire's fetch with the empty finish and holds nothing; the server
//! refused the size the identity states. The vocabulary is the
//! server's — the body carries the full error verbatim, and the
//! container relays it to whoever was waiting rather than
//! interpreting it.

mod request;
mod response;

pub use request::*;
pub use response::*;
