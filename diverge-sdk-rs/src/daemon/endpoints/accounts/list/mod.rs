//! Listing accounts. A client asks for the accounts, narrowed, and the
//! daemon sends every one its `list` grants reach that the request's
//! filter lets through — by name, by identity, by hook resource, by
//! whether it has a name, by the kind of its credential, by the roles
//! it holds, by whether a client is connected as it now, by creator, by
//! tags, by when it was created — one response each, oldest created
//! first, which is the order credentials are tried, and finishes: each
//! the account without its key, its roles, who is connected as it, its
//! tags, when it was created and by whom. A jq program on the request
//! runs over each account the filter lets through, and what it yields
//! is what comes back; a count caps what comes back. A request that
//! says nothing is every account the grants reach. A caller whose
//! grants reach none that matches sees the finish and nothing before
//! it. The daemon does not stay open.
//!
//! Split by who SENDS, as everywhere else. A client asks — so the
//! question is in [`client`] — and the daemon answers, so the answer is
//! in [`server`]. Neither side holds both halves of the exchange.

pub mod client;
pub mod server;
