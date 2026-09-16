//! Why a volume operation failed.

use std::fmt;
use std::io;

use diverge_provider_sdk::shared::error;
use serde_json::json;

use crate::tools;

/// What [`VolumeMountManager`](super::VolumeMountManager) and
/// [`Volume`](super::Volume) fail with.
///
/// The first six are refusals of what was asked, decided before
/// anything is touched; the last three are the host not answering:
/// its filesystem, the formatter, or the resizing tools. A fixed
/// volume's hook that does not answer is not an error: the volume is
/// not listed, as the hook's output documents.
#[derive(Debug)]
pub enum Error {
    /// The name is not one a volume may have — see
    /// [`name::ok`](super::name::ok) — or it is a fixed volume's,
    /// which no identity may create over.
    Name(String),
    /// The identity has a volume by this name already.
    Exists(String),
    /// The identity has no volume by this name.
    Unknown(String),
    /// The volume is a fixed one, and what was asked is done only to
    /// a created volume: a fixed volume is never deleted.
    Fixed(String),
    /// Fewer bytes than an ext4 filesystem with a journal can be
    /// made in.
    TooSmall(u64),
    /// More bytes than the formatter addresses: more 4 KiB blocks
    /// than a `u32` counts, which is 16 TiB.
    TooLarge(u64),
    /// The host's filesystem did not answer.
    Io(io::Error),
    /// The image could not be formatted, or could not be read.
    Format(fstool::Error),
    /// One of the resizing tools — `e2fsck`, `resize2fs` — could not
    /// be started, or ran and refused; see [`tools::Error`]. A shrink
    /// the content fits but the metadata does not ends here, with the
    /// volume as it was.
    Tool(tools::Error),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Name(name) => write!(f, "`{name}` is not a name a volume may have"),
            Error::Exists(name) => write!(f, "a volume named `{name}` exists already"),
            Error::Unknown(name) => write!(f, "no volume named `{name}`"),
            Error::Fixed(name) => write!(f, "`{name}` is a fixed volume"),
            Error::TooSmall(bytes) => write!(f, "{bytes} bytes is too small for a volume"),
            Error::TooLarge(bytes) => write!(f, "{bytes} bytes is too large for a volume"),
            Error::Io(error) => write!(f, "the filesystem did not answer: {error}"),
            Error::Format(error) => write!(f, "the image could not be handled: {error}"),
            Error::Tool(error) => write!(f, "the volume could not be resized: {error}"),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::Io(error) => Some(error),
            Error::Format(error) => Some(error),
            Error::Tool(error) => Some(error),
            Error::Name(_)
            | Error::Exists(_)
            | Error::Unknown(_)
            | Error::Fixed(_)
            | Error::TooSmall(_)
            | Error::TooLarge(_) => None,
        }
    }
}

impl From<io::Error> for Error {
    fn from(error: io::Error) -> Self {
        Error::Io(error)
    }
}

impl From<fstool::Error> for Error {
    fn from(error: fstool::Error) -> Self {
        Error::Format(error)
    }
}

impl From<tools::Error> for Error {
    fn from(error: tools::Error) -> Self {
        Error::Tool(error)
    }
}

/// What the SDK puts on the wire for one of these: the variant's
/// kind, and the message.
///
/// ```json
/// {"kind":"exists","error":"a volume named `work` exists already"}
/// ```
impl From<Error> for error::Error {
    fn from(error: Error) -> Self {
        let kind = match &error {
            Error::Name(_) => "name",
            Error::Exists(_) => "exists",
            Error::Unknown(_) => "unknown",
            Error::Fixed(_) => "fixed",
            Error::TooSmall(_) => "too_small",
            Error::TooLarge(_) => "too_large",
            Error::Io(_) => "io",
            Error::Format(_) => "format",
            Error::Tool(_) => "tool",
        };
        error::Error(json!({
            "kind": kind,
            "error": error.to_string(),
        }))
    }
}
