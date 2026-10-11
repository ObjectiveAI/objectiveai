//! Why a client image could not be listed, examined, built or
//! removed.

use std::fmt;
use std::io;
use std::process::ExitStatus;

use diverge_sdk::shared::error;
use serde_json::json;

use crate::host::tools;
use crate::host::volume_manager;

/// What [`ImageManager`](super::ImageManager) fails with: the store
/// absent, a request refused, or a tool or the host not answering.
#[derive(Debug)]
pub enum Error {
    /// The configuration names no client image store.
    Absent,
    /// The name is not one segment of a repository path.
    Name(String),
    /// The caller has an image by that name already.
    Exists(String),
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
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Absent => f.write_str("this provider keeps no client images"),
            Error::Name(name) => write!(f, "the image name `{name}` is not one segment of a repository path"),
            Error::Exists(name) => write!(f, "an image named `{name}` exists already"),
            Error::Volume(name) => write!(f, "no volume named `{name}`"),
            Error::Volumes(error) => write!(f, "the context volume could not be looked up: {error}"),
            Error::Mount(error) => write!(f, "the context volume could not be attached: {error}"),
            Error::Build(status) => write!(f, "the build ended with {status}"),
            Error::Podman(error) => write!(f, "podman did not answer: {error}"),
            Error::Io(error) => write!(f, "the store could not be read or written: {error}"),
            Error::Json(error) => write!(f, "an image's record did not parse: {error}"),
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
            Error::Absent | Error::Name(_) | Error::Exists(_) | Error::Volume(_) | Error::Build(_) => None,
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
/// {"kind":"exists","error":"an image named `tools` exists already"}
/// ```
impl From<Error> for error::Error {
    fn from(error: Error) -> Self {
        let kind = match &error {
            Error::Absent => "absent",
            Error::Name(_) => "name",
            Error::Exists(_) => "exists",
            Error::Volume(_) => "volume",
            Error::Volumes(_) => "volumes",
            Error::Mount(_) => "mount",
            Error::Build(_) => "build",
            Error::Podman(_) => "podman",
            Error::Io(_) => "io",
            Error::Json(_) => "record",
        };
        error::Error(json!({
            "kind": kind,
            "error": error.to_string(),
        }))
    }
}
