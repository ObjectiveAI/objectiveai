//! Adding a provider: its record, by its identity — the direction it is
//! reached in, what the daemon announces itself with there, whether it
//! accepts other daemons and hears messages through it, whether it is
//! enabled — kept, and an outgoing one dialled at once when enabled.
//! One request, one answer: added, a record of that identity already,
//! forbidden, or a failure — and the scope finishes.
//!
//! Split by who SENDS, as everywhere else. A client asks — so the
//! question is in [`client`] — and the daemon answers, so the answer is
//! in [`server`]. Neither side holds both halves of the exchange.

pub mod client;
pub mod server;
