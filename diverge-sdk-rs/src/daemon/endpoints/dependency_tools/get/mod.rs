//! Getting one dependency tool.
//!
//! One request, one answer. A client names a dependency tool of its
//! own by its template and its owner; the daemon answers with the tool
//! as a list would report it, that no tool of the caller's is the one
//! named, or that it failed, and the scope finishes. A template that
//! is none, and an owner the template has no tool under, are each the
//! `NotFound`.
//!
//! Split by who SENDS, as everywhere else. A client asks — so the
//! question is in [`client`] — and the daemon answers, so the answer
//! is in [`server`]. Neither side holds both halves of the exchange.

pub mod client;
pub mod server;
