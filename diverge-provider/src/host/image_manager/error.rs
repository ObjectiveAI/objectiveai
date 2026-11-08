//! Why the Store could not list, examine, build, push, describe,
//! prove or remove an image, or say whether one is held.

use std::fmt;
use std::io;
use std::path::PathBuf;
use std::process::ExitStatus;

use diverge_sdk::shared::error;
use serde_json::json;

use crate::host::tools;
use crate::host::volume_manager;

/// What [`ImageManager`](super::ImageManager) fails with: the store
/// absent, a request refused, or a tool or the host not answering.
#[derive(Debug)]
pub enum Error {
    /// The configuration names no Store.
    Absent,
    /// The caller has no volume by the name the context gave.
    Volume(String),
    /// The volume manager could not look the context volume up.
    Volumes(volume_manager::Error),
    /// The context volume could not be attached.
    Mount(tools::Error),
    /// The build's tool ended with this status, and nothing was
    /// built; what it said went out as the build's output.
    Build(ExitStatus),
    /// Podman could not be asked something.
    Podman(tools::Error),
    /// A file or directory of the store could not be read or made.
    Io(io::Error),
    /// An image's record did not parse.
    Json(serde_json::Error),
    /// An image's layout has no index, or one that is not one
    /// manifest's.
    Layout(PathBuf),
    /// The operating system's random source did not answer, so no
    /// nonce could be drawn.
    Random(String),
    /// A registry that answered that it serves the image failed to
    /// supply it, or supplied bytes that are not their digest's; or,
    /// with no reference serving the image, a registry could not be
    /// asked whether it does.
    Registry(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Absent => f.write_str("this provider keeps no Store"),
            Error::Volume(name) => write!(f, "no volume named `{name}`"),
            Error::Volumes(error) => write!(f, "the context volume could not be looked up: {error}"),
            Error::Mount(error) => write!(f, "the context volume could not be attached: {error}"),
            Error::Build(status) => write!(f, "the build ended with {status}"),
            Error::Podman(error) => write!(f, "podman did not answer: {error}"),
            Error::Io(error) => write!(f, "the store could not be read or written: {error}"),
            Error::Json(error) => write!(f, "an image's record did not parse: {error}"),
            Error::Layout(dir) => write!(f, "the layout at `{}` could not be read", dir.display()),
            Error::Random(error) => write!(f, "no nonce could be drawn: {error}"),
            Error::Registry(error) => write!(f, "the registry did not answer for the image: {error}"),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::Mount(error) | Error::Podman(error) => Some(error),
            Error::Volumes(error) => Some(error),
            Error::Io(error) => Some(error),
            Error::Json(error) => Some(error),
            Error::Absent | Error::Volume(_) | Error::Build(_) | Error::Layout(_) | Error::Random(_) | Error::Registry(_) => None,
        }
    }
}

impl From<io::Error> for Error {
    fn from(error: io::Error) -> Self {
        Error::Io(error)
    }
}

impl From<serde_json::Error> for Error {
    fn from(error: serde_json::Error) -> Self {
        Error::Json(error)
    }
}

/// What the protocol puts on the wire for one of these.
///
/// ```json
/// {"kind":"registry","error":"the registry did not answer for the image: …"}
/// ```
impl From<Error> for error::Error {
    fn from(error: Error) -> Self {
        let kind = match &error {
            Error::Absent => "absent",
            Error::Volume(_) => "volume",
            Error::Volumes(_) => "volumes",
            Error::Mount(_) => "mount",
            Error::Build(_) => "build",
            Error::Podman(_) => "podman",
            Error::Io(_) => "io",
            Error::Json(_) => "record",
            Error::Layout(_) => "layout",
            Error::Random(_) => "random",
            Error::Registry(_) => "registry",
        };
        error::Error(json!({
            "kind": kind,
            "error": error.to_string(),
        }))
    }
}
