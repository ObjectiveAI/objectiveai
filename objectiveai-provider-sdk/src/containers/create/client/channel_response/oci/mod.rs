//! The registry answers a caller sends back.
//!
//! What rides this channel is the OCI Distribution Specification —
//! the pull half of it. A provider's container runtime issues ordinary
//! registry requests, the provider relays them, and a caller's own
//! registry answers. Nothing on either side is aware of the other.
//!
//! Which is what makes the ordinary things work. `404` is a blob that
//! is not there, said the way HTTP already says it, with no separate
//! signal invented for it and no ambiguity against the empty layer —
//! whose digest is real and whose body is legitimately zero bytes.
//! `206` and `Content-Range` resume an interrupted pull. `HEAD` asks
//! whether a blob exists without fetching it. None of that had to be
//! designed here; it is what a registry already says.
//!
//! # One alias, for what rides the channel
//!
//! [`Frame`] and nothing else. It is an alias because a registry
//! answer is an HTTP response, and
//! [`http::response`](crate::http::response) already says what one is
//! and why it arrives head-first.
//!
//! What the frame CONTAINS is not aliased. Reaching into a
//! [`Frame::Head`](crate::http::response::Frame::Head) means naming
//! [`http::response::Head`](crate::http::response::Head), which is
//! correct: the head is HTTP's, not OCI's, and this module has no
//! opinion about it.
//!
//! Mirroring the target module instead would mean every type added to
//! a shared abstraction has to be re-aliased in every module that
//! borrows it — a can of worms, and one that grows every time the
//! shared thing does.

mod frame;

pub use frame::*;
