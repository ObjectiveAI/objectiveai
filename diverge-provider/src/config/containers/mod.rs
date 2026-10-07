//! The `containers` section of `config.yaml`.
//!
//! The runtime the provider runs containers with, and what it gives
//! it: [`Podman`] — the registries the provider looks in for an
//! image, where podman keeps its data, and the memory and the disk
//! the running set may reach together — and [`ServerImage`], one
//! image the provider holds itself, the first place a run and a
//! check look. [`Containers`] holds them, and is the one section
//! that is never absent: the file may leave it out, and then it is
//! its [`Default`], which is how a provider bootstraps.
//!
//! Its own files are flattened into it, so everything is named
//! through this module and not through the file it lives in.

mod containers;
mod credential;
mod podman;
mod registry;
mod server_image;

pub use containers::*;
pub use credential::*;
pub use podman::*;
pub use registry::*;
pub use server_image::*;
