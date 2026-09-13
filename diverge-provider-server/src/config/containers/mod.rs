//! The `containers` section of `config.yaml`.
//!
//! What the provider gives containers between them: the memory and
//! the disk the running set may reach together, where their storage
//! is kept, and which registries a caller may pull from by name.
//! [`Containers`] holds it, and is the one section that is never
//! absent: the file may leave it out, and then it is its [`Default`],
//! which is how a provider bootstraps. More is coming here — this is
//! the section that will grow.
//!
//! Its own files are flattened into it, so everything is named
//! through this module and not through the file it lives in.

mod containers;
mod credential;
mod registry;

pub use containers::*;
pub use credential::*;
pub use registry::*;
