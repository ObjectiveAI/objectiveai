//! Getting one tool.
//!
//! One request, one answer. A client names a tool of its own; the
//! daemon answers with the tool as a list would report it, that no tool
//! of the caller's is the one named, or that it failed, and the scope
//! finishes. The tool is named by its name, or by its template and its
//! index, as [`reference`](crate::daemon::reference) states.
//!
//! Split by who SENDS, as everywhere else. A client asks — so the
//! question is in [`client`] — and the daemon answers, so the answer
//! is in [`server`]. Neither side holds both halves of the exchange.

pub mod client;
pub mod server;
