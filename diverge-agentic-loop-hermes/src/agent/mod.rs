//! The agent: Hermes's parameters, as the caller states them.
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

pub mod provider;
pub mod toolsets;

mod agent;
mod effort;

pub use agent::*;
pub use effort::*;
pub use provider::Provider;
pub use toolsets::Toolsets;
