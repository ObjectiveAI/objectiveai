//! Editing a tool: a patch to its name, its mounts, its secrets.
//!
//! One request, one answer. A client names a tool of its own and a
//! patch — see [`patch`](crate::daemon::patch) — to the tool's
//! editable members; the daemon applies it, whole or not at all, and
//! answers that the tool is as the result states, that no tool is the
//! one named, that the tool is active and its mounts were left as they
//! are, that the name is another's, or that it failed, and the scope
//! finishes. A connected tool is edited by
//! [`connected_standalone_tools::edit`](crate::daemon::endpoints::connected_standalone_tools::edit),
//! and no request here finds one.
//!
//! Split by who SENDS, as everywhere else. A client asks — so the
//! question is in [`client`] — and the daemon answers, so the answer
//! is in [`server`]. Neither side holds both halves of the exchange.

pub mod client;
pub mod server;
