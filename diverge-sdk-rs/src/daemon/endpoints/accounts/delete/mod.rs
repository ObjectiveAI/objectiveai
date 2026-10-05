//! Deleting an account. One request, one answer. A client names an
//! account; the daemon answers that the account is gone, that no
//! account is the one named, that a container runs under it or a client
//! is connected as it and it was left as it is, forbidden, or that it
//! failed, and the scope finishes. A credential the account accepted is
//! accepted by no account from then on, unless another does; what the
//! account created keeps its creator.
//!
//! Split by who SENDS, as everywhere else. A client asks — so the
//! question is in [`client`] — and the daemon answers, so the answer is
//! in [`server`]. Neither side holds both halves of the exchange.

pub mod client;
pub mod server;
