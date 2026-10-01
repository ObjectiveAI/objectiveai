//! Deleting a resource by id.
//!
//! One request, one answer. A client names a resource of its own by
//! its id; the daemon answers that the resource is deleted, that no
//! resource has that id, that an agent mounts it and it was left as
//! it is, or that it failed, and the scope finishes. A template that
//! mounts a deleted resource makes no agent until the same bytes are
//! uploaded again, which is the same id.
//!
//! Split by who SENDS, as everywhere else. A client asks — so the
//! question is in [`client`] — and the daemon answers, so the answer
//! is in [`server`]. Neither side holds both halves of the exchange.

pub mod client;
pub mod server;
