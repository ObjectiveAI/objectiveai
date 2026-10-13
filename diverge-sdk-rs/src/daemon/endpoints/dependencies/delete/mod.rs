//! Deleting a dependency template by id.
//!
//! One request, one answer. A client names a dependency template of
//! its own by its id; the daemon answers that the template is deleted,
//! that no template has that id, that a tool runs from it and it was
//! left as it is, or that it failed, and the scope finishes. A delete
//! removes the template with every service made from it and every
//! database scope ever made for a tool running from it — under every
//! agent, every agent template, and the global one — and what those
//! scopes held. No agent depends on it after: an agent whose last
//! registration declared it depends on it again when it registers
//! again and declares it, which makes the template anew, with new
//! scopes.
//!
//! Split by who SENDS, as everywhere else. A client asks — so the
//! question is in [`client`] — and the daemon answers, so the answer
//! is in [`server`]. Neither side holds both halves of the exchange.

pub mod client;
pub mod server;
