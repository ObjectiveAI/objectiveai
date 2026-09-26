//! What a server's response frame carries for an agent container run.

use std::fmt;

use super::AgenticLoopChunk;
use crate::wire::decode::Decode;
use crate::wire::encode::{Encode, Writer};
use crate::shared::containers::response::{Id, VolumeHeld};
use crate::shared::error::Error;

/// A run's answer: the container's id, then the agent's conversation
/// for as long as the scope lives — or the volume that refused it, or
/// a failure.
///
/// A payload leads with one byte saying which — `0` for
/// [`Id`](Self::Id), `1` for [`VolumeHeld`](Self::VolumeHeld),
/// `2` for [`Error`](Self::Error), `3` for [`Chunk`](Self::Chunk) —
/// and the rest is that variant's own JSON.
///
/// # The id, then the conversation
///
/// | the scope | means |
/// |-----------|-------|
/// | an id, then chunks, and stays open | the container is running, and the agent is speaking |
/// | an id, then quiet, and stays open | the container is running, and the agent has nothing to say until the next message |
/// | a volume held, then a finish | it never started: that volume is under a stat, an edit or a delete |
/// | an error, then a finish | it never came up, or it is gone |
/// | a finish, with no error | the run is over — a stop, or the container's own end |
///
/// # Many containers per volume, one editor
///
/// A volume may be mounted in any number of containers of its caller
/// at once; what it cannot be is mounted while a stat, an edit or a
/// delete has it to itself. A provider holds every volume a run
/// names, shared, from the moment it accepts the request until the
/// run ends, and a request that names one held exclusively is
/// answered [`VolumeHeld`](Self::VolumeHeld) before anything is
/// fetched or deployed — its own variant, because a caller acts on it
/// differently from a failure: wait for the edit, or name another
/// volume, and ask again. The hold is the volume's
/// [`mount`](crate::provider::server::volume::Volume::mount), taken by the run
/// handler and given back on every ending.
///
/// # The conversation is this stream
///
/// An agent container is one conversation, and this is where it is
/// read. Nothing opens a loop: an
/// [`enqueue`](crate::shared::containers::enqueue) with no loop
/// running starts one on its message, an enqueue while one runs joins
/// the queue, and either way what the agent says arrives here, chunk
/// by chunk, in order, as the proxy sent it. There is no marker
/// between one turn and the next: a message's user parts — see
/// [`user_parts`](super::user_parts) — mark it landing, a
/// [`NotificationChunk`](super::NotificationChunk) with
/// [`is_fatal`](super::NotificationChunk::is_fatal) set marks a loop
/// that died, and quiet is an agent with nothing left to say. The
/// tools family's stream carries nothing after the id: its own
/// exchange answers on channels.
///
/// The tree and the files are still channels the caller opens, so a
/// caller that wants none of them pays for none of them. This stream
/// carries no readiness signal either: a provider knows when a
/// CONTAINER has started, and that is not the same fact as the thing
/// inside it having bound its port. The channels find out, one
/// exchange at a time.
///
/// # The tag is not decoration
///
/// [`AgenticLoopChunk`] is untagged and tells its own variants apart
/// by a `type` constant inside each one, while an [`Error`] is an
/// arbitrary JSON value — including, legitimately, an object with a
/// `type` field. The byte in front is what keeps a provider's error
/// text from being read as a chunk, and a failure from looking like
/// output.
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
    /// A failure. Tag `2`.
    ///
    /// The container is not running and will not be — the image
    /// would not pull, the container would not start, whatever the
    /// provider knows. It is the one variant that ends the scope
    /// rather than adding to it. See
    /// [`shared::error::Error`](crate::shared::error::Error) for why
    /// it says so little. This is not a
    /// [`NotificationChunk`](super::NotificationChunk) with
    /// [`is_fatal`](super::NotificationChunk::is_fatal) set: that is
    /// part of the agent's OUTPUT, a loop saying it is over and why,
    /// and the scope goes on.
    Error(Error),
    /// One chunk of the agent's conversation. Tag `3`.
    ///
    /// After the id, zero or more, for as long as the scope lives, in
    /// the order the agent produced them and exactly as the proxy
    /// sent them; never before the id, and never after a finish. See
    /// [`AgenticLoopChunk`] for what one is.
    Chunk(AgenticLoopChunk),
}

/// Tag for [`Frame::Id`].
const ID: u8 = 0;

/// Tag for [`Frame::VolumeHeld`].
const VOLUME_MOUNTED: u8 = 1;

/// Tag for [`Frame::Error`].
const ERROR: u8 = 2;

/// Tag for [`Frame::Chunk`]. Public, so a relay that carries a
/// chunk's JSON without reading it can frame it.
pub const CHUNK: u8 = 3;

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
                out.extend_from_slice(&[VOLUME_MOUNTED]);
                serde_json::to_writer(out, refused).map_err(FrameEncodeError::VolumeHeld)
            }
            Frame::Error(error) => {
                out.extend_from_slice(&[ERROR]);
                error.encode(out).map_err(FrameEncodeError::Error)
            }
            Frame::Chunk(chunk) => {
                out.extend_from_slice(&[CHUNK]);
                serde_json::to_writer(out, chunk).map_err(FrameEncodeError::Chunk)
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
    VolumeHeld(serde_json::Error),
    /// The error did not serialize.
    Error(serde_json::Error),
    /// The chunk did not serialize.
    Chunk(serde_json::Error),
}

impl fmt::Display for FrameEncodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FrameEncodeError::Id(error) => {
                write!(f, "container id did not serialize: {error}")
            }
            FrameEncodeError::VolumeHeld(error) => {
                write!(f, "volume-mounted refusal did not serialize: {error}")
            }
            FrameEncodeError::Error(error) => {
                write!(f, "agents run error did not serialize: {error}")
            }
            FrameEncodeError::Chunk(error) => {
                write!(f, "agents run chunk did not serialize: {error}")
            }
        }
    }
}

impl std::error::Error for FrameEncodeError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            FrameEncodeError::Id(error) => Some(error),
            FrameEncodeError::VolumeHeld(error) => Some(error),
            FrameEncodeError::Error(error) => Some(error),
            FrameEncodeError::Chunk(error) => Some(error),
        }
    }
}

impl Decode<'_> for Frame {
    /// Six ways to fail, and each names which variant failed.
    type Error = FrameError;

    // Spelled out for the same reason as `encode` above.
    fn decode(bytes: &[u8]) -> Result<Self, FrameError> {
        let (tag, rest) = bytes.split_first().ok_or(FrameError::Empty)?;
        match *tag {
            ID => serde_json::from_slice(rest)
                .map(Frame::Id)
                .map_err(FrameError::Id),
            VOLUME_MOUNTED => serde_json::from_slice(rest)
                .map(Frame::VolumeHeld)
                .map_err(FrameError::VolumeHeld),
            ERROR => Error::decode(rest)
                .map(Frame::Error)
                .map_err(FrameError::Error),
            CHUNK => serde_json::from_slice(rest)
                .map(Frame::Chunk)
                .map_err(FrameError::Chunk),
            tag => Err(FrameError::UnknownTag(tag)),
        }
    }
}

/// An agent container run response frame that could not be read.
#[derive(Debug)]
pub enum FrameError {
    /// No bytes at all, so not even a tag.
    Empty,
    /// A tag that is none of this frame's four.
    UnknownTag(u8),
    /// The id did not parse.
    Id(serde_json::Error),
    /// The refusal did not parse.
    VolumeHeld(serde_json::Error),
    /// The error did not parse.
    Error(serde_json::Error),
    /// The chunk did not parse.
    Chunk(serde_json::Error),
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
            FrameError::VolumeHeld(error) => {
                write!(f, "volume-mounted refusal did not parse: {error}")
            }
            FrameError::Error(error) => {
                write!(f, "agents run error did not parse: {error}")
            }
            FrameError::Chunk(error) => {
                write!(f, "agents run chunk did not parse: {error}")
            }
        }
    }
}

impl std::error::Error for FrameError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            FrameError::Id(error) => Some(error),
            FrameError::VolumeHeld(error) => Some(error),
            FrameError::Error(error) => Some(error),
            FrameError::Chunk(error) => Some(error),
            FrameError::Empty | FrameError::UnknownTag(_) => None,
        }
    }
}
