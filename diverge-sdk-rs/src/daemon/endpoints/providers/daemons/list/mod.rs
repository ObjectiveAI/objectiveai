//! Listing a caller's daemons. A client asks for its daemons, narrowed,
//! and the daemon sends every one it holds under the caller's identity
//! that the request's filter lets through — by name, by a provider it
//! is linked through, by whether this daemon holds a connection to it
//! now, by creator, by tags, all or any, by when it was added — one
//! response each, oldest added first, then the word that the list is
//! whole, and keeps the scope open: each daemon added, changed or
//! removed, as the records and the connections change, until the client
//! cancels, the one channel it opens on the scope. Each is its name,
//! the kind of its mode, its links, whether this daemon is connected to
//! it, when it was added and by whom, and its tags. Never the
//! credential. A count keeps the list to the first that many that
//! match. A request that says nothing is every daemon. A caller with
//! none that matches is told the list is whole at once, and watched.
//!
//! Split by who SENDS, as everywhere else. A client asks — so the
//! question is in [`client`] — and the daemon answers, so the answer is
//! in [`server`]. Neither side holds both halves of the exchange.

pub mod client;
pub mod server;
