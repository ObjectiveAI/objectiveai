//! The agent: a LoNE bundle and its sampling, as the caller states
//! them.
//!
//! This is what the `arguments` on the container's request must
//! be, and every type here derives its JSON Schema so `--schema`
//! can print it — [`schemars::schema_for!`] over [`Agent`] is the
//! whole document, carried by the image's `/DIVERGE.md`, the one
//! place the proxy reads it from. One member is not this image's to
//! define: `dependencies`, the tool containers the agent depends on,
//! each a dependency tool template exactly as the SDK's
//! `shared::containers::dependencies::Template` states one — its
//! schema in `/DIVERGE.md`, the list itself as the registration's
//! answer.
//!
//! No upstream model and no credential. The agent is its one source —
//! a bundle or an endpoint — and every turn runs `lone harness` over
//! the whole conversation, and the chunks it writes are the turn.
//! Reworked from the LoNE agent, keeping what this image honors and
//! dropping what nothing here reads:
//! - `upstream`: the image is the discriminator; a value that reaches
//!   this container is a LoNE agent by arrival.
//! - `memory` and `disk`: ceilings are the container request's
//!   business, set by whoever runs the container, not the agent's
//!   vocabulary. (A tool the agent asks for under `dependencies` names
//!   its own, which are that tool container's.)
//! - `requirements`: nothing is installed at registration; the image
//!   carries the harness, and the bundle is mounted, not built.
//! - Any in-process reach back into the container: the harness calls
//!   tools only through the `assistant_tool_call` chunks it emits,
//!   and reads the answers on its next run.

mod agent;

pub use agent::*;
