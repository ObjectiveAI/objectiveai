//! Registering a tool another daemon holds, under a name.
//!
//! One request, one answer. A client names a daemon it holds an
//! account on — a record of
//! [`providers::daemons`](crate::daemon::endpoints::providers::daemons)
//! — and a tool as that daemon names it, and the name it wants the
//! tool held under; the daemon answers that the tool exists under the
//! name, that the name is already in use, or that it failed, and the
//! scope finishes. Nothing is connected at the request: while an
//! agent the tool is attached to uses it, the daemon connects to the
//! daemon named through a provider both are connected to and opens
//! that daemon's [`connect`](super::connect) naming the tool, over
//! which the tool's MCP exchanges travel, and lets the connection go
//! when nothing has used the tool for `idle_seconds`. The tool's life
//! is not this scope's: it goes on after the finish, reached by its
//! name, attached to agents by an [`attach`](super::attach), until a
//! [`delete`](super::delete).
//!
//! Split by who SENDS, as everywhere else. A client asks — so the
//! question is in [`client`] — and the daemon answers, so the answer
//! is in [`server`]. Neither side holds both halves of the exchange.

pub mod client;
pub mod server;
