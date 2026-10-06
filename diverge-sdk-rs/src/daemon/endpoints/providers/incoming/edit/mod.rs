//! Replacing a credential. One request, one answer. A client names a
//! credential by its identity and gives a credential anew — the
//! identity it names from then on, the address it is accepted from if
//! one; the daemon replaces it whole, mints a new key, and answers the
//! key, once, that no credential names the identity, that the new
//! identity is another credential's, forbidden, or that it failed, and
//! the scope finishes. This is how a key rotates, and how an address or
//! an identity changes. A connection a provider holds through the
//! credential now is not dropped; the next one is judged by the new
//! key.
//!
//! Split by who SENDS, as everywhere else. A client asks — so the
//! question is in [`client`] — and the daemon answers, so the answer is
//! in [`server`]. Neither side holds both halves of the exchange.

pub mod client;
pub mod server;
