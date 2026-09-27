//! Connecting to a tool container somebody else runs, under a name.
//!
//! One request, one answer. A client hands the daemon where a running
//! tool container is — the provider it runs on, its id, and the
//! authorization its runner will judge — and the name it wants the
//! tool held under; the daemon answers that the tool exists under the
//! name, that the name is already in use, or that it failed, and the
//! scope finishes. Nothing is joined at the request: the daemon holds
//! a `containers::tools::connect` scope on that provider while an
//! agent the tool is attached to is active, and lets it go when none
//! is. The tool's life is not this scope's: it goes on after the
//! finish, reached by its name, attached to agents by an
//! [`attach`](super::attach), until a [`delete`](super::delete).
//!
//! Split by who SENDS, as everywhere else. A client asks — so the
//! question is in [`client`] — and the daemon answers, so the answer
//! is in [`server`]. Neither side holds both halves of the exchange.

pub mod client;
pub mod server;
