//! Listing the credentials of incoming providers. A client asks for the
//! credentials, narrowed, and the daemon sends every one its `list`
//! grants reach that the request's filter lets through — by identity,
//! by whether a provider is connected through it now, by creator, by
//! when it was added — one response each, oldest added first, then
//! the word that the list is whole, and keeps the scope open: each
//! credential added, changed or removed, as the records and the
//! connections change, until the client cancels, the one channel it
//! opens on the scope. Each is the credential without its key, who is
//! connected through it, when it was added and by whom. A count keeps
//! the list to the first that many that match. A request that says
//! nothing is every credential the grants reach. A caller whose grants
//! reach none that matches is told the list is whole at once, and
//! watched.
//!
//! Split by who SENDS, as everywhere else. A client asks — so the
//! question is in [`client`] — and the daemon answers, so the answer is
//! in [`server`]. Neither side holds both halves of the exchange.

pub mod client;
pub mod server;
