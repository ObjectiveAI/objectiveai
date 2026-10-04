//! Listing a caller's outgoing providers. A client asks for its
//! outgoing providers, narrowed, and the daemon sends every one it
//! holds under the caller's identity that the request's filter lets
//! through — by address, by the kind of its mode, by whether it is
//! connected now, by creator, by when it was added — one response each,
//! oldest added first, and finishes: each its address, its kind,
//! whether it is connected and when that last changed, when it was
//! added and by whom. Never the credential. A jq program on the request
//! runs over each provider the filter lets through, and what it yields
//! is what comes back; a count caps what comes back. A request that
//! says nothing is every outgoing provider. A caller with none that
//! matches sees the finish and nothing before it. The daemon does not
//! stay open.
//!
//! Split by who SENDS, as everywhere else. A client asks — so the
//! question is in [`client`] — and the daemon answers, so the answer is
//! in [`server`]. Neither side holds both halves of the exchange.

pub mod client;
pub mod server;
