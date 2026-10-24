//! Listing the daemons this daemon reaches. A client asks for them,
//! narrowed — by name, by a provider a daemon is linked through, by
//! whether this daemon holds a connection to it now — and the daemon
//! sends one response each, in the configuration's order, then the
//! word that the list is whole, and keeps the scope open: each daemon
//! changed as its connection comes and goes, until the client cancels,
//! the one channel it opens on the scope. Each is its name, its issuer,
//! its links and whether a connection to it is held. A count keeps the
//! list to the first that many that match.
//!
//! Split by who SENDS, as everywhere else. A client asks — so the
//! question is in [`client`] — and the daemon answers, so the answer is
//! in [`server`]. Neither side holds both halves of the exchange.

pub mod client;
pub mod server;
