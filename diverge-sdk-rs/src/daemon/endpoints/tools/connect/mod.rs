//! Connecting to a tool another daemon holds, under a name.
//!
//! One request, one answer. A client names a daemon it holds an
//! account on — a record of
//! [`providers::daemons`](crate::daemon::endpoints::providers::daemons)
//! — and a tool as that daemon names it, and the name it wants the
//! tool held under; the daemon answers that the tool exists under the
//! name, that the name is already in use, or that it failed, and the
//! scope finishes. Nothing is joined at the request: while an agent
//! the tool is attached to is active, the daemon connects to the
//! daemon named through a provider both are connected to, opens that
//! daemon's [`expose`](super::expose) naming the tool, and joins the
//! container the expose answers with — the provider protocol's
//! `containers::tools::connect` on the provider the expose names, with
//! the id and the authorization it answered — and lets both go when no
//! attached agent is. The tool's life is not this scope's: it goes on
//! after the finish, reached by its name, attached to agents by an
//! [`attach`](super::attach), until a [`delete`](super::delete).
//!
//! Split by who SENDS, as everywhere else. A client asks — so the
//! question is in [`client`] — and the daemon answers, so the answer
//! is in [`server`]. Neither side holds both halves of the exchange.

pub mod client;
pub mod server;
