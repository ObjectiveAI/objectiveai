//! Podman integration — feature `podman`.
//!
//! [`install`] obtains the podman executable (downloaded + installed on
//! first use); [`setup`] owns creating the global podman machine
//! (`machine init` — the macOS VM / Windows WSL2 distro); [`running`] ensures
//! that machine is *running* (start it if stopped, create it if absent) on
//! every use; [`laboratory`] manages laboratory containers. All of it is
//! driven through a [`Podman`] handle — a self-contained runtime rooted at
//! any writable bin directory, so every consumer (the CLI's `Context`, the
//! Tauri viewer, anything else) constructs its own with no host-app types
//! involved.

mod error;
pub use error::*;
mod handle;
pub use handle::*;
pub mod install;
pub mod laboratory;
pub mod running;
pub mod setup;
