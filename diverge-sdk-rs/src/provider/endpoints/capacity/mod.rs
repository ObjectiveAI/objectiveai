//! Asking a provider how much the caller may still use.
//!
//! One question, one answer: a client asks, a provider says four
//! numbers of bytes — the room for a new volume, the room in the
//! client image store, the memory and the overlay disk a container
//! run could still reserve — and the scope finishes. The exchange is
//! shaped as [`info`](super::info) is: an empty request, one JSON
//! response, no failure defined.
//!
//! # What the numbers mean
//!
//! Each is the most of its resource THIS CALLER could use up as of
//! the instant the provider answers, as the provider allots it. A
//! provider that lets every caller use everything it holds answers
//! what it holds; a provider that partitions its callers answers the
//! caller's share, which is the provider's prerogative; neither
//! answers more than it would, at that instant, let the caller have.
//! The numbers are facts about an instant, not reservations: nothing
//! is set aside by answering, and a request that relies on one is
//! answered on its own terms.
//!
//! # Every member is sent; not every member is read
//!
//! A provider answers every request, whatever its
//! [`Capabilities`](super::info::server::response::Capabilities)
//! hold out: a member whose resource the provider does not hold out —
//! `volumes_disk` without a volume mode object, `images_disk` without
//! `images_store` — carries any number, and a caller does not read
//! it. The two container members are always meaningful, since every
//! provider runs containers.
//!
//! Split by who SENDS, as everywhere else. A client asks — so the
//! question is in [`client`] — and a provider answers, so the answer
//! is in [`server`].

pub mod client;
pub mod server;
