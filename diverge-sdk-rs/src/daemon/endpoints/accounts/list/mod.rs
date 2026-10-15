//! Listing accounts. A client asks for the accounts, narrowed, and the
//! daemon sends every one its `list` grants reach that the request's
//! filter lets through — by name, by identity, by whether it has a name
//! or a credential, by the roles it holds, by whether a client is
//! connected as it now, by creator, by tags, by when it was created —
//! one response each, oldest created first, then the word that the
//! list is whole, and keeps the scope open: each account added, changed
//! or removed, as the records, the roles and the connections change,
//! until the client cancels, the one channel it opens on the scope.
//! Each is the account, its credential without its key, its roles,
//! whether a client is connected as it, its tags, when it was created
//! and by whom. A count keeps the list to the first that many that
//! match. A request that says nothing is every account the grants
//! reach. A caller whose grants reach none that matches is told the
//! list is whole at once, and watched.
//!
//! Split by who SENDS, as everywhere else. A client asks — so the
//! question is in [`client`] — and the daemon answers, so the answer is
//! in [`server`]. Neither side holds both halves of the exchange.

pub mod client;
pub mod server;
