//! Spawning an agent under a tag.
//!
//! One request, one answer. A client hands the daemon everything an
//! agent container is made from and the tag it wants the agent held
//! under; the daemon answers that the agent is tagged, that the tag
//! is already in use, or that it failed, and the scope finishes. The
//! agent's life is not this scope's: it goes on after the finish,
//! reached by its tag.
//!
//! Split by who SENDS, as everywhere else. A client asks — so the
//! question is in [`client`] — and the daemon answers, so the answer
//! is in [`server`]. Neither side holds both halves of the exchange.

pub mod client;
pub mod server;
