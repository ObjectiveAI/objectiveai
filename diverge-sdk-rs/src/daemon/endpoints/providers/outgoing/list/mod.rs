//! Listing a caller's outgoing providers. A client asks for its
//! outgoing providers, narrowed, and the daemon sends every one it
//! holds under the caller's identity that the request's filter lets
//! through — by address, by the kind of its mode, by whether it is
//! connected now, by creator, by when it was added — one response each,
//! oldest added first, then the word that the list is whole, and
//! keeps the scope open: each provider added, changed or removed, as
//! the records and the connections change, until the client cancels,
//! the one channel it opens on the scope. Each is its address, its
//! kind, whether it is connected and when that last changed, when it
//! was added and by whom. Never the credential. A count keeps the
//! list to the first that many that match. A request that says
//! nothing is every outgoing provider. A caller with none that
//! matches is told the list is whole at once, and watched.
//!
//! Split by who SENDS, as everywhere else. A client asks — so the
//! question is in [`client`] — and the daemon answers, so the answer is
//! in [`server`]. Neither side holds both halves of the exchange.

pub mod client;
pub mod server;
