//! Podman integration.
//!
//! [`install`] obtains the podman executable (downloaded + installed on
//! first use); [`setup`] owns creating the global podman machine
//! (`machine init` — the macOS VM / Windows WSL2 distro); [`running`] ensures
//! that machine is *running* (start it if stopped, create it if absent) on
//! every use. All three are no-ops or native on Linux and are driven by
//! [`crate::context::Context::podman`], which installs once then ensures the
//! machine is alive. Further podman code — running containers, etc. — will
//! live in sibling modules here.

pub mod install;
pub mod laboratory;
pub mod running;
pub mod setup;
