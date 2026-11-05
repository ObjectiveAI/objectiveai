//! Adding a remote daemon: one account of ours on it. One request, one
//! answer: added, a record of that name already, forbidden, or a
//! failure — the handshake's issuer refused among them where ours and
//! the remote's differ — and the scope finishes.
//!
//! Split by who SENDS, as everywhere else. A client asks — so the
//! question is in [`client`] — and the daemon answers, so the answer is
//! in [`server`]. Neither side holds both halves of the exchange.

pub mod client;
pub mod server;
