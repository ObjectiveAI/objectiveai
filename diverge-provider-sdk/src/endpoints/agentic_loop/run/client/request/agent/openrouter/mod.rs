//! OpenRouter agent parameters.

mod agent;
mod context_compression;
mod provider;
mod reasoning;
mod stop;
mod system_prompt;
mod upstream;
mod verbosity;

pub use agent::*;
pub use context_compression::*;
pub use provider::*;
pub use reasoning::*;
pub use stop::*;
pub use system_prompt::*;
pub use upstream::*;
pub use verbosity::*;
