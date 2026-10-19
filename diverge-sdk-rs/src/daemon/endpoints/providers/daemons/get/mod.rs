//! Getting one daemon. One request, one answer. A client names a daemon
//! of its own by name; the daemon answers with the daemon as a list
//! would report it, that no daemon of the caller's has that name, or
//! that it failed, and the scope finishes.
//!
//! Split by who SENDS, as everywhere else. A client asks — so the
//! question is in [`client`] — and the daemon answers, so the answer is
//! in [`server`]. Neither side holds both halves of the exchange.

pub mod client;
pub mod server;
