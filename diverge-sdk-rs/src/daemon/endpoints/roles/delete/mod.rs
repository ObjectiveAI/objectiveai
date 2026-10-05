//! Deleting a role. One request, one answer. A client names a role; the
//! daemon answers that the role is gone, that no role has the name,
//! that an account holds it and it was left as it is, forbidden, or
//! that it failed, and the scope finishes.
//!
//! Split by who SENDS, as everywhere else. A client asks — so the
//! question is in [`client`] — and the daemon answers, so the answer is
//! in [`server`]. Neither side holds both halves of the exchange.

pub mod client;
pub mod server;
