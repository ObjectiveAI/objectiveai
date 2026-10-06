//! Listing a caller's credentials of incoming providers. A client asks
//! for its credentials, narrowed, and the daemon sends every one it
//! holds under the caller's identity that the request's filter lets
//! through — by the identity a key credential names, by the resource a
//! hook credential is, by kind, by whether a provider is connected
//! through it now, by creator, by when it was added — one response
//! each, oldest added first, which is the order they are tried, and
//! finishes: each the credential without its key, who is connected
//! through it, when it was added and by whom. A jq program on the
//! request runs over each credential the filter lets through, and what it
//! yields is what comes back; a count caps what comes back. A request
//! that says nothing is every credential. A caller with none that
//! matches sees the finish and nothing before it. The daemon does not
//! stay open.
//!
//! Split by who SENDS, as everywhere else. A client asks — so the
//! question is in [`client`] — and the daemon answers, so the answer is
//! in [`server`]. Neither side holds both halves of the exchange.

pub mod client;
pub mod server;
