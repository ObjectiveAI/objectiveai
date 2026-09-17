//! Running a container: the SDK's `ContainerDeployer` and
//! `Container`, supplied by this crate, with podman.
//!
//! # What a deploy is
//!
//! One `podman run`, and what has to be true before and after it. A
//! container is RUNNING and its proxy LISTENING when a deploy
//! returns, so the steps are ordered and every one of them is undone
//! when a later one fails: the container's `disk` and `memory` are
//! taken from the two caps the configuration sets, and refused when
//! the running set would pass one; every volume mount is resolved
//! through the [`VolumeManager`](crate::volume_manager::VolumeManager)
//! and, for a stored volume, loop-mounted on a directory of the
//! provider's; the image is pulled, unless it is one of the
//! provider's own; the image cache is measured and trimmed; the
//! container is started with the proxy binary bound in, port `14979`
//! published to a loopback port podman picks, `/dev/fuse` and the
//! privilege to mount, and no environment but the request's; the
//! proxy is started inside it with `podman exec` and the deploy waits
//! until it answers on the published port.
//!
//! # The proxy
//!
//! `diverge-container-proxy`, a static Linux program, beside the
//! provider's own executable. It is bound read-only at
//! `/.diverge/diverge-container-proxy` inside every container and
//! started as a second process there, beside the image's own
//! entrypoint; the `podman exec` that starts it is held for the
//! container's life, so its end is seen. The proxy reads no
//! environment and takes no argument.
//!
//! # The machine
//!
//! On macOS and Windows podman runs in a machine, and the deployer
//! brings it to what the configuration says before anything else is
//! asked of podman: made if there is none, its disk under
//! `storage_path`, podman inside it as root, on macOS its memory the
//! configured `memory`, and running. A machine already made with its
//! disk elsewhere is refused, with the path, for the operator to
//! remove.
//!
//! # The images
//!
//! A caller-held image is pulled from the provider's own registry,
//! which listens on this host's loopback: on Linux podman pulls on
//! the host and reaches it there; on macOS and Windows podman pulls
//! inside its machine, and reaches the registry through one SSH
//! tunnel the provider opens into the machine with the machine's own
//! settings, for the provider's life. A `server` image is one the
//! configuration lists, run as `<name>@<digest>` with nothing pulled.
//! A registry image is pulled as named, from a host the configuration
//! lists, with the credentials it lists, through an auth file the
//! provider writes for podman.
//!
//! Podman evicts nothing from its image store, so the provider does:
//! every image present when the provider starts is protected and
//! counted, and what the provider pulled since is removed, least
//! recently run first and only while no container runs it, whenever
//! the store's count passes `image_cache_disk` after a pull. A store
//! still over the cap after that refuses the run.
//!
//! # What is here
//!
//! [`ContainerDeployer`] is the deployer, [`Container`] a container it
//! started, [`Error`] what a deploy fails with. [`Limit`] is one cap
//! with a count against it, [`Images`] the image cache's bookkeeping,
//! [`Source`] where an image comes from, and `deploy`, `mounts` and,
//! on the hosts with a machine, `machine` and `tunnel` the steps.
//!
//! Its own files are flattened into it, so everything is named
//! through this module and not through the file it lives in.

mod container;
mod container_deployer;
mod deploy;
mod error;
mod images;
mod limit;
#[cfg(not(target_os = "linux"))]
mod machine;
mod mounts;
mod shared;
mod source;
#[cfg(not(target_os = "linux"))]
mod tunnel;

pub use container::*;
pub use container_deployer::*;
pub use error::*;
pub use images::*;
pub use limit::*;
pub use shared::*;
pub use source::*;
#[cfg(not(target_os = "linux"))]
pub use tunnel::*;
