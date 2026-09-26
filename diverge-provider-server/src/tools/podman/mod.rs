//! Podman, and the machine it runs in.
//!
//! On Linux podman is the host's, and the host's filesystem is the
//! one a container's mounts come from. On macOS and Windows podman
//! runs inside a machine — a Linux VM, Fedora CoreOS — and the host's
//! directories reach it as mounts of the VM's, under paths of the
//! VM's own; a tool that must run where the images are runs inside
//! the machine, over `podman machine ssh`. [`command`] is the root
//! every podman invocation grows from, told once by [`configure`]
//! where podman's data is, and [`podman`] runs one; [`images`],
//! [`image_id`], [`manifest_exists`], [`port`] and [`containers`]
//! are the questions the provider asks podman, each read out of
//! podman's own output;
//! [`sudo`], [`path`] and [`machine`], on the hosts with a machine,
//! are a tool run inside it as root, a host path as it sees it, and
//! the machine as podman describes it.
//!
//! Its own files are flattened into it, so everything is named
//! through this module and not through the file it lives in.

mod command;
mod inspect;
#[cfg(not(target_os = "linux"))]
mod machine;

pub use command::*;
pub use inspect::*;
#[cfg(not(target_os = "linux"))]
pub use machine::*;
