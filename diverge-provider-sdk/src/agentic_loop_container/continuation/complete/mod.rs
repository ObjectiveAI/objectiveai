//! Sealing the continuation's delivery: every chunk is in.
//!
//! `POST /continuation/complete` — what lets the container act on
//! the continuation: until it arrives, more bytes may follow. A
//! completion with NO chunks before it is the fresh start — the
//! caller had nothing to resume from, and the run begins here. No
//! identity backs the bytes, so whether a delivery is whole is the
//! container's own to judge when it opens them.
//!
//! # A settlement, one of two
//!
//! The completion and the [`error`](super::error) are the delivery's
//! two endings, and it gets exactly one: once settled, it takes
//! nothing further — no chunks, no second completion, no error
//! (`409`).

mod request;
mod response;

pub use request::*;
pub use response::*;
