//! Making a template.
//!
//! One request, one answer. A client hands the daemon a template; the
//! daemon answers with the template's id — the hash of what was
//! handed — saying whether it was made or already was, or that it
//! failed, and the scope finishes. Two identical templates are one:
//! a create of one that exists is not a failure, and answers the same
//! id.
//!
//! Split by who SENDS, as everywhere else. A client asks — so the
//! question is in [`client`] — and the daemon answers, so the answer
//! is in [`server`]. Neither side holds both halves of the exchange.

pub mod client;
pub mod server;
