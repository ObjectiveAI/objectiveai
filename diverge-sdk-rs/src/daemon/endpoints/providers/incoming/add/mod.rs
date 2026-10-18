//! Adding a credential of incoming providers. One request, one answer.
//! A client gives the identity a provider presenting the credential
//! will have, and the address it is accepted from if one; the daemon
//! mints a key, and answers it, once, that a credential for that
//! identity exists already, forbidden, or that it failed, and the scope
//! finishes. The key is never reported again.
//!
//! Split by who SENDS, as everywhere else. A client asks — so the
//! question is in [`client`] — and the daemon answers, so the answer is
//! in [`server`]. Neither side holds both halves of the exchange.

pub mod client;
pub mod server;
