//! Listing roles. A client asks for the roles, narrowed, and the daemon
//! sends every one its `list` grants reach that the request's filter
//! lets through — by name, by the accounts that hold it, by creator, by
//! tags, by when it was created — one response each, oldest created
//! first, then the word that the list is whole, and keeps the scope
//! open: each role added, changed or removed, as the roles and the
//! accounts holding them change, until the client cancels, the one
//! channel it opens on the scope. Each is the role with its grants, the
//! accounts holding it, its tags, when it was created and by whom. A
//! count keeps the list to the first that many that match. A request
//! that says nothing is every role the grants reach. A caller whose
//! grants reach none that matches is told the list is whole at once,
//! and watched.
//!
//! Split by who SENDS, as everywhere else. A client asks — so the
//! question is in [`client`] — and the daemon answers, so the answer is
//! in [`server`]. Neither side holds both halves of the exchange.

pub mod client;
pub mod server;
