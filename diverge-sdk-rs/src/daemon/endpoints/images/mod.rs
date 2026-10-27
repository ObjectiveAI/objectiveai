//! Images: named by digest, never held.
//!
//! An image is no record of the daemon's. A request names one by its
//! [`Image`](crate::shared::containers::request::Image) — the digest
//! that is the image, and the references that say where its bytes may
//! be — and the daemon reaches it through what it does hold: a
//! container that runs it, or a provider that can obtain it.
//!
//! [`info`] is the one request: the image's DIVERGE.md, which says what
//! the arguments of a container of it may be. Over images a grant names
//! nothing, `{"images":["info"]}`: see
//! [`grant::images`](crate::daemon::grant::images).

pub mod info;
