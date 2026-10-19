//! Listing vault keys. A client asks for the keys, narrowed, and the
//! daemon sends every one its `list` grants reach that the request's
//! filter lets through — by name, by creator, by tags, by when it was
//! made — one response each, oldest first, then the word that the
//! list is whole, and keeps the scope open: each key set anew,
//! changed or deleted, as the keys, their slots' locks and the
//! records binding them change, until the client cancels, the one
//! channel it opens on the scope. Each is the key with its slots —
//! each by number, and who holds it — its tags, the agents and tools
//! binding it, when it was made and by whom — and never a value,
//! which a [`get`](super::get) alone answers. A count keeps the list
//! to the first that many that match. A request that says nothing is
//! every key the grants reach. A caller whose grants reach none that
//! matches is told the list is whole at once, and watched.
//!
//! Split by who SENDS, as everywhere else. A client asks — so the
//! question is in [`client`] — and the daemon answers, so the answer is
//! in [`server`]. Neither side holds both halves of the exchange.

pub mod client;
pub mod server;
