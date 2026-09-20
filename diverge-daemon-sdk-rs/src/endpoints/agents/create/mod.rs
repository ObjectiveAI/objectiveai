//! Creating an agent under a name.
//!
//! One request, one answer. A client hands the daemon everything an
//! agent container is made from and the name it wants the agent held
//! under; the daemon answers that the agent is created, that the name
//! is already in use, or that it failed, and the scope finishes. The
//! agent's life is not this scope's: it goes on after the finish,
//! reached by its name, until a [`delete`](super::delete).
//!
//! Split by who SENDS, as everywhere else. A client asks — so the
//! question is in [`client`] — and the daemon answers, so the answer
//! is in [`server`]. Neither side holds both halves of the exchange.

pub mod client;
pub mod server;
