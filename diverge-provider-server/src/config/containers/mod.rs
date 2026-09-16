//! The `containers` section of `config.yaml`.
//!
//! The runtime the provider runs containers with, and what it gives
//! it: [`Podman`] — the registries a caller may pull from by name,
//! where podman keeps its data, and the memory and the disk the
//! running set may reach together. [`Containers`] holds it, and is the
//! one section that is never absent: the file may leave it out, and
//! then it is its [`Default`], which is how a provider bootstraps.
//! More is coming here — this is the section that will grow.
//!
//! Its own files are flattened into it, so everything is named
//! through this module and not through the file it lives in.

mod containers;
mod credential;
mod podman;
mod registry;

pub use containers::*;
pub use credential::*;
pub use podman::*;
pub use registry::*;
