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
//! # Aliases
//!
//! Nothing is defined here. A registry answer is an HTTP response, and
//! [`http::response`](crate::http::response) already says what one is
//! and why it arrives head-first.
//!
//! Aliases rather than a re-export of that module, because this one is
//! real: a reader looking for what a caller answers a creation with
//! finds it under the path that says so, and each alias names its own
//! target in its own signature.

mod frame;
mod head;

pub use frame::*;
pub use head::*;
