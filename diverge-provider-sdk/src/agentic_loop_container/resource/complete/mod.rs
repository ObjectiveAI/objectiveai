//! Sealing a resource's delivery: every chunk is in.
//!
//! `POST /resource/{identity}/complete` — what lets the container
//! act on the resource: until it arrives, more bytes may follow.
//! Whether the whole is right is the identity's own promise — the
//! size and hash it carries are exactly what a short or wrong
//! delivery fails. A lone completion with no chunks before it
//! delivers the empty resource.
//!
//! # A settlement, one of two
//!
//! The completion and the [`error`](super::error) are the
//! delivery's two endings, and an identity gets exactly one: once
//! settled, it takes nothing further — no chunks, no second
//! completion, no error (`409`).

mod request;
mod response;

pub use request::*;
pub use response::*;
