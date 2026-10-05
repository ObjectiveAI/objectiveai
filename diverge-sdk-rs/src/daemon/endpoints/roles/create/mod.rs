//! Creating a role. One request, one answer. A client gives a name,
//! what the role is for in words, and its grants; the daemon answers
//! that the role is created, that the name is a role's already,
//! forbidden, or that it failed, and the scope finishes. The grants are
//! the caller's to write; the daemon reads them no further than
//! decoding them, and a grant wider than the caller's own is a role the
//! caller holds no `grant` grant over until somebody who does gives it.
//!
//! Split by who SENDS, as everywhere else. A client asks — so the
//! question is in [`client`] — and the daemon answers, so the answer is
//! in [`server`]. Neither side holds both halves of the exchange.

pub mod client;
pub mod server;
