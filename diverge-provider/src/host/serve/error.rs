//! Why the provider could not start, or could not listen.

use std::fmt;
use std::io;
use std::path::PathBuf;

use diverge_sdk::file_lock;
use diverge_sdk::identities;

use crate::config;

use crate::host::container_deployer;
use crate::host::image_registry;

/// What [`run`](super::run) fails with: the configuration, the root's
/// lock, what it names on disk, a piece that could not be made, or
/// the port. A `main` returning one of
/// these prints it, and that is the one report a failed start gets,
/// so its `Debug` is its `Display`.
pub enum Error {
    /// The runtime could not be built.
    Runtime(io::Error),
    /// The root or the file could not be used.
    Config(config::Error),
    /// The root's lock file could not be taken, written or read.
    Lock(file_lock::Error),
    /// The root's identities could not be opened.
    Identities(identities::Error),
    /// The key-set cache could not be made.
    KeySets(identities::VerifyError),
    /// Another provider holds the root: the root, and the process id
    /// the holder left in the lock file, where it left one.
    Locked {
        /// The root held.
        root: PathBuf,
        /// The holder's process id, as it wrote it.
        holder: Option<String>,
    },
    /// A directory of the provider's could not be made.
    Directory(PathBuf, io::Error),
    /// A fixed volume's path is not an existing directory.
    FixedMissing(PathBuf),
    /// A store's capacity is `0`.
    Capacity(PathBuf),
    /// The client image store's capacity is `0`.
    ImageCapacity(PathBuf),
    /// A fixed volume's name is not one a volume may have, or two
    /// fixed volumes have it.
    FixedName(String),
    /// The image registry could not start.
    Registry(image_registry::Error),
    /// The deployer could not be made: podman, its machine, the proxy
    /// binary, or a file under `run/`.
    Deployer(container_deployer::Error),
    /// The port could not be bound.
    Bind(io::Error),
    /// The listener stopped on its own.
    Serve(io::Error),
    /// The scratch directory for ephemeral serves could not be made
    /// or swept.
    Scratch(io::Error),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Runtime(error) => write!(f, "the runtime could not be built: {error}"),
            Error::Config(error) => write!(f, "the configuration could not be used: {error}"),
            Error::Lock(error) => write!(f, "the root could not be locked: {error}"),
            Error::Identities(error) => write!(f, "the identities could not be opened: {error}"),
            Error::KeySets(error) => write!(f, "the key-set cache could not be made: {error}"),
            Error::Locked { root, holder } => match holder {
                Some(holder) => write!(f, "another provider holds {}: process {holder}", root.display()),
                None => write!(f, "another provider holds {}", root.display()),
            },
            Error::Directory(path, error) => write!(f, "`{}` could not be made: {error}", path.display()),
            Error::FixedMissing(path) => write!(f, "the fixed volume `{}` is not a directory that exists", path.display()),
            Error::Capacity(path) => write!(f, "the store `{}` has a capacity of 0", path.display()),
            Error::ImageCapacity(path) => write!(f, "the image store `{}` has a capacity of 0", path.display()),
            Error::FixedName(name) => write!(f, "the fixed volume name `{name}` is not usable, or is used twice"),
            Error::Registry(error) => write!(f, "the image registry could not start: {error}"),
            Error::Deployer(error) => write!(f, "the deployer could not be made: {error}"),
            Error::Bind(error) => write!(f, "the port could not be bound: {error}"),
            Error::Serve(error) => write!(f, "the listener stopped: {error}"),
            Error::Scratch(error) => write!(f, "the scratch directory could not be made ready: {error}"),
        }
    }
}

impl fmt::Debug for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::Runtime(error) => Some(error),
            Error::Config(error) => Some(error),
            Error::Lock(error) => Some(error),
            Error::Identities(error) => Some(error),
            Error::KeySets(error) => Some(error),
            Error::Locked { .. } => None,
            Error::Directory(_, error) => Some(error),
            Error::FixedMissing(_) | Error::Capacity(_) | Error::ImageCapacity(_) | Error::FixedName(_) => None,
            Error::Registry(error) => Some(error),
            Error::Deployer(error) => Some(error),
            Error::Bind(error) => Some(error),
            Error::Serve(error) => Some(error),
            Error::Scratch(error) => Some(error),
        }
    }
}

impl From<config::Error> for Error {
    fn from(error: config::Error) -> Self {
        Error::Config(error)
    }
}
