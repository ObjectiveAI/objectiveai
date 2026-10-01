//! Creating a tool under a name.
//!
//! One request, one answer. A client names the
//! [`template`](super::templates) a tool is made from, the mounts and
//! the provider that are the tool's own, and the name it wants the
//! tool held under; the daemon answers that the tool is created, that the name
//! is already in use, or that it failed, and the scope finishes. The
//! tool's life is not this scope's: it goes on after the finish,
//! reached by its name, attached to agents by an
//! [`attach`](super::attach), until a [`delete`](super::delete).
//! Nothing runs at a create: the tool container runs while an agent
//! it is attached to is active.
//!
//! Split by who SENDS, as everywhere else. A client asks — so the
//! question is in [`client`] — and the daemon answers, so the answer
//! is in [`server`]. Neither side holds both halves of the exchange.

pub mod client;
pub mod server;
