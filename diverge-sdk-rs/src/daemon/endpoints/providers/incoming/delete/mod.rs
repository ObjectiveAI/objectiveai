//! Taking a judge out. One request, one answer. A client names a judge
//! of its own; the daemon answers that the judge is gone, that no judge
//! is the one named, that a provider is connected through it and it was
//! left as it is, or that it failed, and the scope finishes. A
//! credential the judge accepted is accepted by no judge from then on,
//! unless another does.
//!
//! Split by who SENDS, as everywhere else. A client asks — so the
//! question is in [`client`] — and the daemon answers, so the answer is
//! in [`server`]. Neither side holds both halves of the exchange.

pub mod client;
pub mod server;
