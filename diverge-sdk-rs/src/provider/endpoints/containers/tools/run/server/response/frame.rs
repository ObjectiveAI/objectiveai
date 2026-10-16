//! What a server's response frame carries for a tool container run.

use std::fmt;

use crate::wire::decode::Decode;
use crate::wire::encode::{Encode, Writer};
use crate::shared::containers::response::{
    Id, VolumeFuseServed, VolumeHeld, VolumeMode, VolumeMountUnsupported, VolumeMounted,
};
use crate::shared::error::Error;

/// A run's answer: the container's id — or the volume that refused
/// it, or a failure.
///
/// A payload leads with one byte saying which — `0` for
/// [`Id`](Self::Id), `1` for [`VolumeHeld`](Self::VolumeHeld), `2`
/// for [`VolumeMounted`](Self::VolumeMounted), `3` for
/// [`VolumeFuseServed`](Self::VolumeFuseServed), `4` for
/// [`VolumeMountUnsupported`](Self::VolumeMountUnsupported), `5` for
/// [`VolumeMode`](Self::VolumeMode), `6` for
/// [`Error`](Self::Error) — and the rest is that variant's own JSON.
///
/// # Silence is the good case
///
/// | the scope | means |
/// |-----------|-------|
/// | an id, then nothing, and stays open | the container is running |
/// | a volume held, then a finish | it never started: that volume is under a stat, a read, a write, a filetree, an edit or a delete |
/// | a volume mounted, then a finish | it never started: a running container has that volume, and the provider mounts a volume of its mode once |
/// | a volume FUSE served, then a finish | it never started: a FUSE serve has that volume, and the provider does not mount a volume of its mode beside one |
/// | a volume mount unsupported, then a finish | it never started: the provider does not mount a volume of that volume's mode |
/// | a volume mode, then a finish | it never started: that volume is not in the mode its mount names |
/// | FUSE mounts unsupported, then a finish | it never started: the request names a FUSE mount, and the provider mounts none |
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
/// answer. Who else may hold the volume meanwhile is the provider's
/// capability for the volume's mode, stated in its
/// [`info`](crate::provider::endpoints::info): a running container
/// that has it refuses the run, [`VolumeMounted`](Self::VolumeMounted),
/// on a provider whose `mount_many` for the mode is `false`; a FUSE
/// serve that has it refuses the run,
/// [`VolumeFuseServed`](Self::VolumeFuseServed), where
/// `mount_and_fuse_serve` is `false`; a mode the provider does not
/// mount at all refuses it,
/// [`VolumeMountUnsupported`](Self::VolumeMountUnsupported); and no
/// volume is mounted while a stat, a read, a write, a filetree, an
/// edit or a delete has it to itself: a request that names one so
/// held is answered [`VolumeHeld`](Self::VolumeHeld). Each
/// refusal is its own variant, because a caller acts on it
/// differently from a failure: edit the volume or the mount, stop the
/// container or the FUSE serve, wait for the operation, name another
/// volume or ask another provider, and ask again. The hold is the
/// volume's `mount`, taken by the run handler and given back on every
/// ending.
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
    /// The run was refused: a volume it names is under a stat, a
    /// read, a write, a filetree, an edit or a delete. Tag `1`.
    ///
    /// The first and only response of its scope; the finish follows.
    /// Nothing was fetched and nothing was deployed.
    VolumeHeld(VolumeHeld),
    /// The run was refused: a running container has a volume it
    /// names, and the provider mounts a volume of that mode once.
    /// Tag `2`.
    ///
    /// The first and only response of its scope; the finish follows.
    /// Nothing was fetched and nothing was deployed.
    VolumeMounted(VolumeMounted),
    /// The run was refused: a FUSE serve has a volume it names, and
    /// the provider does not mount a volume of that mode beside one.
    /// Tag `3`.
    ///
    /// The first and only response of its scope; the finish follows.
    /// Nothing was fetched and nothing was deployed.
    VolumeFuseServed(VolumeFuseServed),
    /// The run was refused: the provider does not mount a volume of
    /// the mode a volume it names is in. Tag `4`.
    ///
    /// The first and only response of its scope; the finish follows.
    /// Nothing was fetched and nothing was deployed.
    VolumeMountUnsupported(VolumeMountUnsupported),
    /// The run was refused: a volume it names is not in the mode the
    /// mount means it to have. Tag `5`.
    ///
    /// The first and only response of its scope; the finish follows.
    /// Nothing was fetched and nothing was deployed, and the volume is
    /// as it was.
    VolumeMode(VolumeMode),
    /// A failure. Tag `6`.
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

/// Tag for [`Frame::VolumeHeld`].
const VOLUME_HELD: u8 = 1;

/// Tag for [`Frame::VolumeMounted`].
const VOLUME_MOUNTED: u8 = 2;

/// Tag for [`Frame::VolumeFuseServed`].
const VOLUME_FUSE_SERVED: u8 = 3;

/// Tag for [`Frame::VolumeMountUnsupported`].
const VOLUME_MOUNT_UNSUPPORTED: u8 = 4;

/// Tag for [`Frame::VolumeMode`].
const VOLUME_MODE: u8 = 5;

/// Tag for [`Frame::Error`].
const ERROR: u8 = 6;

impl Encode for Frame {
    /// One failure per variant that carries JSON.
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
            Frame::VolumeMounted(refused) => {
                out.extend_from_slice(&[VOLUME_MOUNTED]);
                serde_json::to_writer(out, refused).map_err(FrameEncodeError::VolumeMounted)
            }
            Frame::VolumeFuseServed(refused) => {
                out.extend_from_slice(&[VOLUME_FUSE_SERVED]);
                serde_json::to_writer(out, refused).map_err(FrameEncodeError::VolumeFuseServed)
            }
            Frame::VolumeMountUnsupported(refused) => {
                out.extend_from_slice(&[VOLUME_MOUNT_UNSUPPORTED]);
                serde_json::to_writer(out, refused).map_err(FrameEncodeError::VolumeMountUnsupported)
            }
            Frame::VolumeMode(refused) => {
                out.extend_from_slice(&[VOLUME_MODE]);
                serde_json::to_writer(out, refused).map_err(FrameEncodeError::VolumeMode)
            }
            Frame::Error(error) => {
                out.extend_from_slice(&[ERROR]);
                error.encode(out).map_err(FrameEncodeError::Error)
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
    /// The mounted refusal did not serialize.
    VolumeMounted(serde_json::Error),
    /// The FUSE-served refusal did not serialize.
    VolumeFuseServed(serde_json::Error),
    /// The mount-unsupported refusal did not serialize.
    VolumeMountUnsupported(serde_json::Error),
    /// The mode refusal did not serialize.
    VolumeMode(serde_json::Error),
    /// The error did not serialize.
    Error(serde_json::Error),
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
            FrameEncodeError::VolumeMounted(error) => {
                write!(f, "volume-mounted refusal did not serialize: {error}")
            }
            FrameEncodeError::VolumeFuseServed(error) => {
                write!(f, "volume-fuse-served refusal did not serialize: {error}")
            }
            FrameEncodeError::VolumeMountUnsupported(error) => {
                write!(f, "volume-mount-unsupported refusal did not serialize: {error}")
            }
            FrameEncodeError::VolumeMode(error) => {
                write!(f, "volume-mode refusal did not serialize: {error}")
            }
            FrameEncodeError::Error(error) => {
                write!(f, "tools run error did not serialize: {error}")
            }
        }
    }
}

impl std::error::Error for FrameEncodeError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            FrameEncodeError::Id(error) => Some(error),
            FrameEncodeError::VolumeHeld(error) => Some(error),
            FrameEncodeError::VolumeMounted(error) => Some(error),
            FrameEncodeError::VolumeFuseServed(error) => Some(error),
            FrameEncodeError::VolumeMountUnsupported(error) => Some(error),
            FrameEncodeError::VolumeMode(error) => Some(error),
            FrameEncodeError::Error(error) => Some(error),
        }
    }
}

impl Decode<'_> for Frame {
    /// Eight ways to fail, and each names which variant failed.
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
            VOLUME_MOUNTED => serde_json::from_slice(rest)
                .map(Frame::VolumeMounted)
                .map_err(FrameError::VolumeMounted),
            VOLUME_FUSE_SERVED => serde_json::from_slice(rest)
                .map(Frame::VolumeFuseServed)
                .map_err(FrameError::VolumeFuseServed),
            VOLUME_MOUNT_UNSUPPORTED => serde_json::from_slice(rest)
                .map(Frame::VolumeMountUnsupported)
                .map_err(FrameError::VolumeMountUnsupported),
            VOLUME_MODE => serde_json::from_slice(rest)
                .map(Frame::VolumeMode)
                .map_err(FrameError::VolumeMode),
            ERROR => Error::decode(rest)
                .map(Frame::Error)
                .map_err(FrameError::Error),
            tag => Err(FrameError::UnknownTag(tag)),
        }
    }
}

/// A tool container run response frame that could not be read.
#[derive(Debug)]
pub enum FrameError {
    /// No bytes at all, so not even a tag.
    Empty,
    /// A tag that is none of this frame's seven.
    UnknownTag(u8),
    /// The id did not parse.
    Id(serde_json::Error),
    /// The held refusal did not parse.
    VolumeHeld(serde_json::Error),
    /// The mounted refusal did not parse.
    VolumeMounted(serde_json::Error),
    /// The FUSE-served refusal did not parse.
    VolumeFuseServed(serde_json::Error),
    /// The mount-unsupported refusal did not parse.
    VolumeMountUnsupported(serde_json::Error),
    /// The mode refusal did not parse.
    VolumeMode(serde_json::Error),
    /// The error did not parse.
    Error(serde_json::Error),
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
            FrameError::VolumeMounted(error) => {
                write!(f, "volume-mounted refusal did not parse: {error}")
            }
            FrameError::VolumeFuseServed(error) => {
                write!(f, "volume-fuse-served refusal did not parse: {error}")
            }
            FrameError::VolumeMountUnsupported(error) => {
                write!(f, "volume-mount-unsupported refusal did not parse: {error}")
            }
            FrameError::VolumeMode(error) => {
                write!(f, "volume-mode refusal did not parse: {error}")
            }
            FrameError::Error(error) => {
                write!(f, "tools run error did not parse: {error}")
            }
        }
    }
}

impl std::error::Error for FrameError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            FrameError::Id(error) => Some(error),
            FrameError::VolumeHeld(error) => Some(error),
            FrameError::VolumeMounted(error) => Some(error),
            FrameError::VolumeFuseServed(error) => Some(error),
            FrameError::VolumeMountUnsupported(error) => Some(error),
            FrameError::VolumeMode(error) => Some(error),
            FrameError::Error(error) => Some(error),
            FrameError::Empty | FrameError::UnknownTag(_) => None,
        }
    }
}
