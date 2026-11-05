//! Deleting an account. One request, one answer. A client names an
//! account; the daemon answers that the account is gone — every
//! connection a client had as it closed — that no account is the one
//! named, forbidden, or that it failed, and the scope finishes. A credential the account accepted is
//! accepted by no account from then on, unless another does; what the
//! account created keeps its creator.
//!
//! Split by who SENDS, as everywhere else. A client asks — so the
//! question is in [`client`] — and the daemon answers, so the answer is
//! in [`server`]. Neither side holds both halves of the exchange.

pub mod client;
pub mod server;
