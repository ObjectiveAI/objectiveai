//! Podman integration.
//!
//! [`install`] obtains the podman executable (downloaded + installed on
//! first use); [`setup`] initializes the global podman machine
//! (`machine init` — the macOS VM / Windows WSL2 distro; no-op on Linux).
//! Both are gated for concurrency and driven by
//! [`crate::context::Context::podman`], which runs install then setup.
//! Further podman code — running containers, etc. — will live in sibling
//! modules here.

pub mod install;
pub mod setup;
