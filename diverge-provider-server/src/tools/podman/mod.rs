//! Podman, and the machine it runs in.
//!
//! On Linux podman is the host's, and the host's filesystem is the
//! one a container's mounts come from. On macOS and Windows podman
//! runs inside a machine — a Linux VM, Fedora CoreOS — and the host's
//! directories reach it as mounts of the VM's, under paths of the
//! VM's own; a tool that must run where the images are runs inside
//! the machine, over `podman machine ssh`. [`command`] is the root
//! every podman invocation grows from and [`podman`] runs one;
//! [`sudo`] and [`path`], on the hosts with a machine, are a tool
//! run inside it as root and a host path as it sees it.
//!
//! Its own files are flattened into it, so everything is named
//! through this module and not through the file it lives in.

mod command;
#[cfg(not(target_os = "linux"))]
mod machine;

pub use command::*;
#[cfg(not(target_os = "linux"))]
pub use machine::*;
