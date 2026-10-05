//! Editing a tool: its name, its account, its mounts, its deployer.
//!
//! One request, one answer. A client names a tool of its own and
//! states anew whichever of those it names — each replaced whole, the
//! rest as they are — and the daemon answers that the tool has them,
//! that no tool is the one named, that the tool is active and its
//! mounts were left as they are, that the tool is connected and has
//! no mounts or account of this caller's to change, that the name is
//! another's, that the account named is none the daemon has, or that
//! it failed, and the scope finishes.
//!
//! Split by who SENDS, as everywhere else. A client asks — so the
//! question is in [`client`] — and the daemon answers, so the answer
//! is in [`server`]. Neither side holds both halves of the exchange.

pub mod client;
pub mod server;
