//! The agent: OpenRouter's parameters, as the caller states them.
//!
//! This is what the `arguments` on the container's request must
//! be, and every type here derives its JSON Schema so `GET /schema`
//! can say so — [`schemars::schema_for!`] over [`Agent`] is the
//! whole answer. One member is not this image's to define:
//! `dependencies`, the tool containers the agent depends on, each a
//! dependency tool template in the form the provider's wire gives one,
//! passed back whole as the registration's answer.

mod agent;
mod context_compression;
mod provider;
mod reasoning;
mod stop;
mod verbosity;

pub use agent::*;
pub use context_compression::*;
pub use provider::*;
pub use reasoning::*;
pub use stop::*;
pub use verbosity::*;
