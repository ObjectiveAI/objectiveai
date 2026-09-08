//! The agent: OpenRouter's parameters, as the caller states them.
//!
//! This is what the `agent` value on the container's request must
//! be, and every type here derives its JSON Schema so `GET /schema`
//! can say so — [`schemars::schema_for!`] over [`Agent`] is the
//! whole answer.

mod agent;
mod context_compression;
mod provider;
mod reasoning;
mod stop;
mod upstream;
mod verbosity;

pub use agent::*;
pub use context_compression::*;
pub use provider::*;
pub use reasoning::*;
pub use stop::*;
pub use upstream::*;
pub use verbosity::*;
