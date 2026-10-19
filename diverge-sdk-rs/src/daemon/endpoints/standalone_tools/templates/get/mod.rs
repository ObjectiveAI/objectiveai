//! Getting one template.
//!
//! One request, one answer. A client names a template of its own; the
//! daemon answers with the template as a list would report it, that no
//! template of the caller's is the one named, or that it failed, and
//! the scope finishes. The template is named by its id, its hash.
//!
//! Split by who SENDS, as everywhere else. A client asks — so the
//! question is in [`client`] — and the daemon answers, so the answer
//! is in [`server`]. Neither side holds both halves of the exchange.

pub mod client;
pub mod server;
