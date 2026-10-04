//! Adding a judge of incoming providers. One request, one answer. A
//! client gives a judge — a key with the identity it names, or a hook
//! resource; the daemon answers that the judge is added, that one of
//! its kind for that identity or that resource exists already, that the
//! hook names no resource the caller holds or not one that is a hook,
//! or that it failed, and the scope finishes. The judge is appended:
//! tried after every judge there was.
//!
//! Split by who SENDS, as everywhere else. A client asks — so the
//! question is in [`client`] — and the daemon answers, so the answer is
//! in [`server`]. Neither side holds both halves of the exchange.

pub mod client;
pub mod server;
