//! Deleting a template by id.
//!
//! One request, one answer. A client names a template of its own by
//! its id; the daemon answers that the template is deleted, that no
//! template has that id, that an agent was made from it and it was
//! left as it is, or that it failed, and the scope finishes.
//!
//! Split by who SENDS, as everywhere else. A client asks — so the
//! question is in [`client`] — and the daemon answers, so the answer
//! is in [`server`]. Neither side holds both halves of the exchange.

pub mod client;
pub mod server;
