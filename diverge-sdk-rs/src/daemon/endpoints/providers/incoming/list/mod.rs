//! Listing the credentials of incoming providers. A client asks for the
//! credentials, narrowed, and the daemon sends every one its `list`
//! grants reach that the request's filter lets through — by identity,
//! by whether a provider is connected through it now, by creator, by
//! when it was added — one response each, oldest added first, and
//! finishes: each the credential without its key, who is connected
//! through it, when it was added and by whom. A count caps what comes
//! back. A request that says nothing is every credential the grants
//! reach. A caller whose grants reach none that matches sees the finish
//! and nothing before it. The daemon does not stay open.
//!
//! Split by who SENDS, as everywhere else. A client asks — so the
//! question is in [`client`] — and the daemon answers, so the answer is
//! in [`server`]. Neither side holds both halves of the exchange.

pub mod client;
pub mod server;
