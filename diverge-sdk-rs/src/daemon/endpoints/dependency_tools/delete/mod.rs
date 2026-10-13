//! Deleting a dependency template's tools, by the template's id.
//!
//! One request, one answer. A client names a dependency template by
//! its id; the daemon answers that the template's tools are deleted,
//! that no tool has that template, that one of them runs and they were
//! left as they are, or that it failed, and the scope finishes. A
//! delete removes every tool of the template under every owner — each
//! agent's, each agent template's, and the global one — with each
//! tool's database scope and what it held, and the template itself.
//! No agent depends on any of them after: an agent whose last
//! registration declared the template depends on it again when it
//! registers again and declares it, which makes its tool anew, with a
//! new scope.
//!
//! Split by who SENDS, as everywhere else. A client asks — so the
//! question is in [`client`] — and the daemon answers, so the answer
//! is in [`server`]. Neither side holds both halves of the exchange.

pub mod client;
pub mod server;
