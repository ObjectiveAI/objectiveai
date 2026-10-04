//! Getting one judge. One request, one answer. A client names a judge
//! of its own — a key judge by its identity, a hook judge by its
//! resource; the daemon answers with the judge as a list would report
//! it, without its key, that no judge is the one named, or that it
//! failed, and the scope finishes.
//!
//! Split by who SENDS, as everywhere else. A client asks — so the
//! question is in [`client`] — and the daemon answers, so the answer is
//! in [`server`]. Neither side holds both halves of the exchange.

pub mod client;
pub mod server;
