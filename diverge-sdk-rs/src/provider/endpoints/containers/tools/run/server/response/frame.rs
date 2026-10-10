//! What a server's response frame carries for a tool container run.

use std::fmt;

use crate::wire::decode::Decode;
use crate::wire::encode::{Encode, Writer};
use super::Connector;
use crate::shared::containers::response::{Id, VolumeHeld, VolumeMode};
use crate::shared::error::Error;

/// A run's answer: the container's id, and then, for as long as the
/// scope lives, every connector attached to the container and every
/// one leaving it — or the volume that refused it, or a failure.
///
/// A payload leads with one byte saying which — `0` for
/// [`Id`](Self::Id), `1` for [`VolumeHeld`](Self::VolumeHeld),
/// `2` for [`VolumeMode`](Self::VolumeMode), `3` for
/// [`Error`](Self::Error), `4` for [`Connected`](Self::Connected),
/// `5` for [`Disconnected`](Self::Disconnected) — and the rest is
/// that variant's own JSON.
///
/// # Silence is the good case
///
/// | the scope | means |
/// |-----------|-------|
/// | an id, then nothing, and stays open | the container is running, and nobody is connected to it |
/// | an id, then a connected, a disconnected, as they come | the container is running, and this is who is on it |
/// | a volume held, then a finish | it never started: that volume is under a stat, an edit or a delete, or, persistent, has its one user |
/// | a volume mode, then a finish | it never started: that volume is not in the mode its mount names |
/// | an error, then a finish | it never came up, or it is gone |
/// | a finish, with no error | the run is over — a stop, or the container's own end |
///
/// # One mode, and who may hold it
///
/// A mount states the mode it means the volume to have, and a volume
/// in another mode refuses the run,
/// [`VolumeMode`](Self::VolumeMode), with the mode it is in — before
/// anything is fetched or deployed, and after the hold is taken and
/// given back, so an edit cannot slip between the look and the
/// answer. An ephemeral or a read-only volume may be mounted in any
/// number of containers of its caller at once; a persistent one has
/// one user at a time; and no volume is mounted while a stat, an edit
/// or a delete has it to itself: a request that names one held is
/// answered [`VolumeHeld`](Self::VolumeHeld). Each refusal is its
/// own variant, because a caller acts on it differently from a
/// failure: edit the volume or the mount, wait for the edit, or name
/// another volume, and ask again. The hold is the volume's
/// [`mount`](crate::provider::server::volume::Volume::mount), taken by the run
/// handler and given back on every ending.
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
    /// The run was refused: a volume it names is under a stat, an
    /// edit or a delete. Tag `1`.
    ///
    /// The first and only response of its scope; the finish follows.
    /// Nothing was fetched and nothing was deployed.
    VolumeHeld(VolumeHeld),
    /// The run was refused: a volume it names is not in the mode the
    /// mount means it to have. Tag `2`.
    ///
    /// The first and only response of its scope; the finish follows.
    /// Nothing was fetched and nothing was deployed, and the volume is
    /// as it was.
    VolumeMode(VolumeMode),
    /// A failure. Tag `3`.
    ///
    /// The container is not running and will not be — the image
    /// would not pull, the container would not start, whatever the
    /// provider knows. It is the one variant that ends the scope
    /// rather than adding to it. See
    /// [`shared::error::Error`](crate::shared::error::Error) for why
    /// it says so little.
    Error(Error),
    /// A connector is attached to the container. Tag `4`.
    ///
    /// After the id, one per connect scope the provider admitted,
    /// sent once the connector is attached and before any of its
    /// channels is served. See [`Connector`].
    Connected(Connector),
    /// A connector's scope on the container has ended. Tag `5`.
    ///
    /// One per `Connected`, however the connect scope ended: the
    /// connector's disconnect, its connection going, or the run's
    /// own end — in which case the finish follows them all.
    Disconnected(Connector),
}

/// Tag for [`Frame::Id`].
const ID: u8 = 0;

/// Tag for [`Frame::VolumeHeld`].
const VOLUME_HELD: u8 = 1;

/// Tag for [`Frame::VolumeMode`].
const VOLUME_MODE: u8 = 2;

/// Tag for [`Frame::Error`].
const ERROR: u8 = 3;

/// Tag for [`Frame::Connected`].
const CONNECTED: u8 = 4;

/// Tag for [`Frame::Disconnected`].
const DISCONNECTED: u8 = 5;

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
            Frame::VolumeHeld(refused) => {
                out.extend_from_slice(&[VOLUME_HELD]);
                serde_json::to_writer(out, refused).map_err(FrameEncodeError::VolumeHeld)
            }
            Frame::VolumeMode(refused) => {
                out.extend_from_slice(&[VOLUME_MODE]);
                serde_json::to_writer(out, refused).map_err(FrameEncodeError::VolumeMode)
            }
            Frame::Error(error) => {
                out.extend_from_slice(&[ERROR]);
                error.encode(out).map_err(FrameEncodeError::Error)
            }
            Frame::Connected(connector) => {
                out.extend_from_slice(&[CONNECTED]);
                serde_json::to_writer(out, connector).map_err(FrameEncodeError::Connector)
            }
            Frame::Disconnected(connector) => {
                out.extend_from_slice(&[DISCONNECTED]);
                serde_json::to_writer(out, connector).map_err(FrameEncodeError::Connector)
            }
        }
    }
}

/// A tool container run response that could not be written.
#[derive(Debug)]
pub enum FrameEncodeError {
    /// The id did not serialize.
    Id(serde_json::Error),
    /// The held refusal did not serialize.
    VolumeHeld(serde_json::Error),
    /// The mode refusal did not serialize.
    VolumeMode(serde_json::Error),
    /// The error did not serialize.
    Error(serde_json::Error),
    /// A connector did not serialize.
    Connector(serde_json::Error),
}

impl fmt::Display for FrameEncodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FrameEncodeError::Id(error) => {
                write!(f, "container id did not serialize: {error}")
            }
            FrameEncodeError::VolumeHeld(error) => {
                write!(f, "volume-held refusal did not serialize: {error}")
            }
            FrameEncodeError::VolumeMode(error) => {
                write!(f, "volume-mode refusal did not serialize: {error}")
            }
            FrameEncodeError::Error(error) => {
                write!(f, "tools run error did not serialize: {error}")
            }
            FrameEncodeError::Connector(error) => {
                write!(f, "connector did not serialize: {error}")
            }
        }
    }
}

impl std::error::Error for FrameEncodeError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            FrameEncodeError::Id(error) => Some(error),
            FrameEncodeError::VolumeHeld(error) => Some(error),
            FrameEncodeError::VolumeMode(error) => Some(error),
            FrameEncodeError::Error(error) => Some(error),
            FrameEncodeError::Connector(error) => Some(error),
        }
    }
}

impl Decode<'_> for Frame {
    /// Seven ways to fail, and each names which variant failed.
    type Error = FrameError;

    // Spelled out for the same reason as `encode` above.
    fn decode(bytes: &[u8]) -> Result<Self, FrameError> {
        let (tag, rest) = bytes.split_first().ok_or(FrameError::Empty)?;
        match *tag {
            ID => serde_json::from_slice(rest)
                .map(Frame::Id)
                .map_err(FrameError::Id),
            VOLUME_HELD => serde_json::from_slice(rest)
                .map(Frame::VolumeHeld)
                .map_err(FrameError::VolumeHeld),
            VOLUME_MODE => serde_json::from_slice(rest)
                .map(Frame::VolumeMode)
                .map_err(FrameError::VolumeMode),
            ERROR => Error::decode(rest)
                .map(Frame::Error)
                .map_err(FrameError::Error),
            CONNECTED => serde_json::from_slice(rest)
                .map(Frame::Connected)
                .map_err(FrameError::Connector),
            DISCONNECTED => serde_json::from_slice(rest)
                .map(Frame::Disconnected)
                .map_err(FrameError::Connector),
            tag => Err(FrameError::UnknownTag(tag)),
        }
    }
}

/// A tool container run response frame that could not be read.
#[derive(Debug)]
pub enum FrameError {
    /// No bytes at all, so not even a tag.
    Empty,
    /// A tag that is none of this frame's six.
    UnknownTag(u8),
    /// The id did not parse.
    Id(serde_json::Error),
    /// The held refusal did not parse.
    VolumeHeld(serde_json::Error),
    /// The mode refusal did not parse.
    VolumeMode(serde_json::Error),
    /// The error did not parse.
    Error(serde_json::Error),
    /// A connector did not parse.
    Connector(serde_json::Error),
}

impl fmt::Display for FrameError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FrameError::Empty => {
                f.write_str("tools run response frame is empty")
            }
            FrameError::UnknownTag(tag) => {
                write!(f, "unknown tools run response frame tag {tag}")
            }
            FrameError::Id(error) => {
                write!(f, "container id did not parse: {error}")
            }
            FrameError::VolumeHeld(error) => {
                write!(f, "volume-held refusal did not parse: {error}")
            }
            FrameError::VolumeMode(error) => {
                write!(f, "volume-mode refusal did not parse: {error}")
            }
            FrameError::Error(error) => {
                write!(f, "tools run error did not parse: {error}")
            }
            FrameError::Connector(error) => {
                write!(f, "connector did not parse: {error}")
            }
        }
    }
}

impl std::error::Error for FrameError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            FrameError::Id(error) => Some(error),
            FrameError::VolumeHeld(error) => Some(error),
            FrameError::VolumeMode(error) => Some(error),
            FrameError::Error(error) => Some(error),
            FrameError::Connector(error) => Some(error),
            FrameError::Empty | FrameError::UnknownTag(_) => None,
        }
    }
}
