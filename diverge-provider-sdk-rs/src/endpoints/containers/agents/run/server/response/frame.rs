//! What a server's response frame carries for an agent container run.

use std::fmt;

use crate::decode::Decode;
use crate::encode::{Encode, Writer};
use crate::shared::containers::response::{Id, VolumeMounted};
use crate::shared::error::Error;

/// A run's answer: the container's id, and then nothing, for as long
/// as the scope lives — or the volume that refused it, or a failure.
///
/// A payload leads with one byte saying which — `0` for
/// [`Id`](Self::Id), `1` for [`VolumeMounted`](Self::VolumeMounted),
/// `2` for [`Error`](Self::Error) — and the rest is that variant's
/// own JSON.
///
/// # Silence is the good case
///
/// | the scope | means |
/// |-----------|-------|
/// | an id, then nothing, and stays open | the container is running |
/// | a volume mounted, then a finish | it never started: that volume is in another container of the caller's |
/// | an error, then a finish | it never came up, or it is gone |
/// | a finish, with no error | the run is over — a stop, or the container's own end |
///
/// # One container per volume
///
/// A volume is mounted in at most one container of its caller at a
/// time. A provider holds every volume a run names from the moment it
/// accepts the request until the run ends, and a request that names
/// a held one is answered [`VolumeMounted`](Self::VolumeMounted)
/// before anything is fetched or deployed — its own variant, because
/// a caller acts on it differently from a failure: stop the other
/// container, or name another volume, and ask again.
///
/// Everything a caller reads from the container — the tree, the
/// family's own exchange — is a channel it opens, not this stream.
/// Which is why this carries no readiness signal either: a provider
/// knows when a CONTAINER has started, and that is not the same fact
/// as the thing inside it having bound its port. The channels find
/// out, one exchange at a time.
#[derive(Debug, Clone, PartialEq)]
pub enum Frame {
    /// The container's id. Tag `0`.
    ///
    /// Arrives once, whenever the provider has it. See [`Id`].
    Id(Id),
    /// The run was refused: a volume it names is mounted in another
    /// container of the caller's. Tag `1`.
    ///
    /// The first and only response of its scope; the finish follows.
    /// Nothing was fetched and nothing was deployed.
    VolumeMounted(VolumeMounted),
    /// A failure. Tag `2`.
    ///
    /// The container is not running and will not be — the image
    /// would not pull, the container would not start, whatever the
    /// provider knows. It is the one variant that ends the scope
    /// rather than adding to it. See
    /// [`shared::error::Error`](crate::shared::error::Error) for why
    /// it says so little.
    Error(Error),
}

/// Tag for [`Frame::Id`].
const ID: u8 = 0;

/// Tag for [`Frame::VolumeMounted`].
const VOLUME_MOUNTED: u8 = 1;

/// Tag for [`Frame::Error`].
const ERROR: u8 = 2;

impl Encode for Frame {
    /// One failure per variant, and all are JSON's.
    type Error = FrameEncodeError;

    // Spelled out rather than `Self::Error`: this enum has a variant
    // called `Error`, so the associated type is ambiguous by that name.
    fn encode(
        &self,
        out: &mut Writer<'_>,
    ) -> Result<(), FrameEncodeError> {
        match self {
            Frame::Id(id) => {
                out.extend_from_slice(&[ID]);
                serde_json::to_writer(out, id).map_err(FrameEncodeError::Id)
            }
            Frame::VolumeMounted(refused) => {
                out.extend_from_slice(&[VOLUME_MOUNTED]);
                serde_json::to_writer(out, refused).map_err(FrameEncodeError::VolumeMounted)
            }
            Frame::Error(error) => {
                out.extend_from_slice(&[ERROR]);
                error.encode(out).map_err(FrameEncodeError::Error)
            }
        }
    }
}

/// An agent container run response that could not be written.
#[derive(Debug)]
pub enum FrameEncodeError {
    /// The id did not serialize.
    Id(serde_json::Error),
    /// The refusal did not serialize.
    VolumeMounted(serde_json::Error),
    /// The error did not serialize.
    Error(serde_json::Error),
}

impl fmt::Display for FrameEncodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FrameEncodeError::Id(error) => {
                write!(f, "container id did not serialize: {error}")
            }
            FrameEncodeError::VolumeMounted(error) => {
                write!(f, "volume-mounted refusal did not serialize: {error}")
            }
            FrameEncodeError::Error(error) => {
                write!(f, "agents run error did not serialize: {error}")
            }
        }
    }
}

impl std::error::Error for FrameEncodeError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            FrameEncodeError::Id(error) => Some(error),
            FrameEncodeError::VolumeMounted(error) => Some(error),
            FrameEncodeError::Error(error) => Some(error),
        }
    }
}

impl Decode<'_> for Frame {
    /// Five ways to fail, and each names which variant failed.
    type Error = FrameError;

    // Spelled out for the same reason as `encode` above.
    fn decode(bytes: &[u8]) -> Result<Self, FrameError> {
        let (tag, rest) = bytes.split_first().ok_or(FrameError::Empty)?;
        match *tag {
            ID => serde_json::from_slice(rest)
                .map(Frame::Id)
                .map_err(FrameError::Id),
            VOLUME_MOUNTED => serde_json::from_slice(rest)
                .map(Frame::VolumeMounted)
                .map_err(FrameError::VolumeMounted),
            ERROR => Error::decode(rest)
                .map(Frame::Error)
                .map_err(FrameError::Error),
            tag => Err(FrameError::UnknownTag(tag)),
        }
    }
}

/// An agent container run response frame that could not be read.
#[derive(Debug)]
pub enum FrameError {
    /// No bytes at all, so not even a tag.
    Empty,
    /// A tag that is none of this frame's three.
    UnknownTag(u8),
    /// The id did not parse.
    Id(serde_json::Error),
    /// The refusal did not parse.
    VolumeMounted(serde_json::Error),
    /// The error did not parse.
    Error(serde_json::Error),
}

impl fmt::Display for FrameError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FrameError::Empty => {
                f.write_str("agents run response frame is empty")
            }
            FrameError::UnknownTag(tag) => {
                write!(f, "unknown agents run response frame tag {tag}")
            }
            FrameError::Id(error) => {
                write!(f, "container id did not parse: {error}")
            }
            FrameError::VolumeMounted(error) => {
                write!(f, "volume-mounted refusal did not parse: {error}")
            }
            FrameError::Error(error) => {
                write!(f, "agents run error did not parse: {error}")
            }
        }
    }
}

impl std::error::Error for FrameError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            FrameError::Id(error) => Some(error),
            FrameError::VolumeMounted(error) => Some(error),
            FrameError::Error(error) => Some(error),
            FrameError::Empty | FrameError::UnknownTag(_) => None,
        }
    }
}
