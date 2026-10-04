//! Replacing a judge. One request, one answer. A client names a judge
//! of its own and gives a judge of the same kind anew — a key judge a
//! new key, address or identity, a hook judge a new resource; the
//! daemon answers that the judge is replaced, that no judge is the one
//! named, that the judge given is not of the named one's kind, or that
//! it failed, and the scope finishes. The judge keeps its place in the
//! order; a key judge given a new identity is named by the new one from
//! then on.
//!
//! Split by who SENDS, as everywhere else. A client asks — so the
//! question is in [`client`] — and the daemon answers, so the answer is
//! in [`server`]. Neither side holds both halves of the exchange.

pub mod client;
pub mod server;
