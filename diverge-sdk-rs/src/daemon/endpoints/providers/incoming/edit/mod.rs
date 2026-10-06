//! Replacing a credential. One request, one answer. A client names a
//! credential of its own and gives a credential of the same kind anew —
//! a key credential a new key, address or identity, a hook credential a
//! new resource; the daemon answers that the credential is replaced,
//! that no credential is the one named, that the credential given is
//! not of the named one's kind, or that it failed, and the scope
//! finishes. The credential keeps its place in the order; a key
//! credential given a new identity is named by the new one from then
//! on.
//!
//! Split by who SENDS, as everywhere else. A client asks — so the
//! question is in [`client`] — and the daemon answers, so the answer is
//! in [`server`]. Neither side holds both halves of the exchange.

pub mod client;
pub mod server;
