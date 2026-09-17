//! Why a container is not running, or why the deployer could not be
//! made.

use std::fmt;
use std::io;
use std::path::PathBuf;

use diverge_provider_sdk::shared::error;
use serde_json::json;

use crate::tools;
use crate::volume_manager;

/// What [`ContainerDeployer`](super::ContainerDeployer) fails with:
/// a request refused before anything ran, a tool that refused, or
/// what could not be made when the provider started.
#[derive(Debug)]
pub enum Error {
    /// The image name is not a repository path: a segment that is
    /// not lowercase letters, digits and single separators, or an
    /// empty one. Refused, never normalized, since it lands in a URL.
    Name(String),
    /// The `server` image named is not one the configuration lists.
    NotOffered {
        /// The repository path asked for.
        name: String,
        /// The digest asked for.
        digest: String,
    },
    /// The reference names a registry host the configuration does not
    /// list.
    Registry(String),
    /// The reference names no host and the configuration lists no
    /// registry to read it under.
    NoRegistry,
    /// The running containers' `disk` would pass
    /// `container_overlay_disk` with this one.
    Disk,
    /// The running containers' `memory` would pass `memory` with this
    /// one.
    Memory,
    /// No volume by this name for the identity.
    Volume(String),
    /// The volume manager could not look a volume up.
    Volumes(volume_manager::Error),
    /// A mount path refused, and why: the root, a component that is
    /// not a name, two mounts at one path, or one path inside
    /// another.
    Path(String),
    /// A volume's image could not be loop-mounted.
    Mount(tools::Error),
    /// The image could not be pulled.
    Pull(tools::Error),
    /// The image store is over `image_cache_disk` with this image in
    /// it, and nothing more can be removed.
    ImageCache,
    /// `podman run` refused.
    Run(tools::Error),
    /// The proxy inside the container ended before it answered.
    Proxy,
    /// Podman could not be asked something.
    Podman(tools::Error),
    /// The tunnel into the podman machine could not be opened, or is
    /// gone.
    Tunnel,
    /// The host's `ssh` could not be started for the tunnel.
    Ssh(tools::Error),
    /// A file of the provider's could not be written or read.
    Io(io::Error),
    /// The proxy binary is not beside the provider's executable.
    Missing(PathBuf),
    /// The podman machine's disk is not under `storage_path`, and
    /// this is where it is; the machine is the operator's to remove.
    Machine(PathBuf),
    /// On Linux, the provider is not running as root, and podman run
    /// from its account would not be rootful.
    Root,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Name(name) => write!(f, "the image name `{name}` is not a repository path"),
            Error::NotOffered { name, digest } => {
                write!(f, "the provider offers no image `{name}` at `{digest}`")
            }
            Error::Registry(host) => write!(f, "the registry `{host}` is not one the provider pulls from"),
            Error::NoRegistry => write!(f, "the reference names no registry and the provider lists none"),
            Error::Disk => write!(f, "the running containers have the provider's disk"),
            Error::Memory => write!(f, "the running containers have the provider's memory"),
            Error::Volume(name) => write!(f, "no volume named `{name}`"),
            Error::Volumes(error) => write!(f, "the volume could not be looked up: {error}"),
            Error::Path(why) => write!(f, "a mount path is refused: {why}"),
            Error::Mount(error) => write!(f, "the volume could not be mounted: {error}"),
            Error::Pull(error) => write!(f, "the image could not be pulled: {error}"),
            Error::ImageCache => write!(f, "the image cache is full"),
            Error::Run(error) => write!(f, "the container could not be started: {error}"),
            Error::Proxy => write!(f, "the proxy ended before it listened"),
            Error::Podman(error) => write!(f, "podman could not be asked: {error}"),
            Error::Tunnel => write!(f, "the tunnel into the podman machine is not open"),
            Error::Ssh(error) => write!(f, "the tunnel could not be started: {error}"),
            Error::Io(error) => write!(f, "a file of the provider could not be used: {error}"),
            Error::Missing(path) => write!(f, "the proxy is not at `{}`", path.display()),
            Error::Machine(path) => write!(
                f,
                "the podman machine's disk is at `{}`, not under the storage path; remove the machine",
                path.display()
            ),
            Error::Root => write!(f, "the provider is not root, and podman on this host is run as the provider"),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::Name(_) => None,
            Error::NotOffered { .. } => None,
            Error::Registry(_) => None,
            Error::NoRegistry => None,
            Error::Disk => None,
            Error::Memory => None,
            Error::Volume(_) => None,
            Error::Volumes(error) => Some(error),
            Error::Path(_) => None,
            Error::Mount(error) => Some(error),
            Error::Pull(error) => Some(error),
            Error::ImageCache => None,
            Error::Run(error) => Some(error),
            Error::Proxy => None,
            Error::Podman(error) => Some(error),
            Error::Tunnel => None,
            Error::Ssh(error) => Some(error),
            Error::Io(error) => Some(error),
            Error::Missing(_) => None,
            Error::Machine(_) => None,
            Error::Root => None,
        }
    }
}

impl From<io::Error> for Error {
    fn from(error: io::Error) -> Self {
        Error::Io(error)
    }
}

impl From<volume_manager::Error> for Error {
    fn from(error: volume_manager::Error) -> Self {
        Error::Volumes(error)
    }
}

/// What the SDK puts on the wire for one of these.
///
/// ```json
/// {"kind":"disk","error":"the running containers have the provider's disk"}
/// ```
impl From<Error> for error::Error {
    fn from(error: Error) -> Self {
        let kind = match &error {
            Error::Name(_) => "name",
            Error::NotOffered { .. } => "not_offered",
            Error::Registry(_) => "registry",
            Error::NoRegistry => "no_registry",
            Error::Disk => "disk",
            Error::Memory => "memory",
            Error::Volume(_) => "volume",
            Error::Volumes(_) => "volumes",
            Error::Path(_) => "path",
            Error::Mount(_) => "mount",
            Error::Pull(_) => "pull",
            Error::ImageCache => "image_cache",
            Error::Run(_) => "run",
            Error::Proxy => "proxy",
            Error::Podman(_) => "podman",
            Error::Tunnel => "tunnel",
            Error::Ssh(_) => "ssh",
            Error::Io(_) => "io",
            Error::Missing(_) => "missing",
            Error::Machine(_) => "machine",
            Error::Root => "root",
        };
        error::Error(json!({
            "kind": kind,
            "error": error.to_string(),
        }))
    }
}
