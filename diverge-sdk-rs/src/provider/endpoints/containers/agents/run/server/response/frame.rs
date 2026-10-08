//! What a server's response frame carries for an agent container run.

use std::fmt;

use super::AgenticLoopChunk;
use crate::wire::decode::Decode;
use crate::wire::encode::{Encode, Writer};
use crate::shared::containers::response::{Id, VolumeHeld, VolumeMode};
use crate::shared::error::Error;

/// A run's answer: the container's id, then the agent's conversation
/// for as long as the scope lives, each loop of it bracketed by the
/// proxy's own word that it began and that it ended — or the volume
/// that refused it, or a failure.
///
/// A payload leads with one byte saying which — `0` for
/// [`Id`](Self::Id), `1` for [`VolumeHeld`](Self::VolumeHeld),
/// `2` for [`VolumeMode`](Self::VolumeMode), `3` for
/// [`Error`](Self::Error), `4` for [`Chunk`](Self::Chunk), `5` for
/// [`Active`](Self::Active), `6` for [`Inactive`](Self::Inactive) —
/// and the rest is that variant's own
/// JSON, or nothing: the two words carry nothing after the byte.
///
/// # The id, then the conversation
///
/// | the scope | means |
/// |-----------|-------|
/// | an id, then quiet, and stays open | the container is running, and no loop has run |
/// | an id, an active, then chunks, and stays open | the container is running, a loop is running, and the agent is speaking |
/// | …an active, chunks, an inactive, and stays open | that loop ran and ended; the agent is quiet until the next message |
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
/// # The conversation is this stream
///
/// An agent container is one conversation, and this is where it is
/// read. Nothing opens a loop: an
/// [`enqueue`](crate::shared::containers::enqueue) with no loop
/// running starts one on its message, an enqueue while one runs joins
/// the queue, and either way what the agent says arrives here, chunk
/// by chunk, in order, as the proxy sent it. A message's user parts —
/// see [`user_parts`](super::user_parts) — mark it landing, and a
/// [`NotificationChunk`](super::NotificationChunk) with
/// [`is_fatal`](super::NotificationChunk::is_fatal) set marks a loop
/// that died. The tools family's stream carries nothing after the id:
/// its own exchange answers on channels.
///
/// # The proxy marks the loop, and only the proxy can
///
/// A loop is one `POST /run` on the agent's server inside the
/// container: it begins when that call answers `2xx` and ends when the
/// event stream of that answer ends, however it ends. The proxy makes
/// the call, so the proxy is the one party that knows both moments,
/// and it says them on its begin stream — see
/// [the begin's frame](crate::container_proxy::outside::endpoints::agents::begin::server::response::Frame)
/// — which the provider relays here as [`Active`](Self::Active) before
/// the loop's first chunk and [`Inactive`](Self::Inactive) after its
/// last. Between an `Inactive` and the next `Active` the agent says
/// nothing; quiet between an `Active` and its `Inactive` is an agent
/// still working. So a caller knows whether the agent is busy from
/// the stream alone, and never guesses it from silence.
///
/// The tree and the files are still channels the caller opens, so a
/// caller that wants none of them pays for none of them. This stream
/// carries no readiness signal: a provider knows when a CONTAINER has
/// started, and that is not the same fact as the thing inside it
/// having bound its port. The channels find out, one exchange at a
/// time. The three facts are three things — the container up, which
/// is the id; the program reachable, which the channels learn; and a
/// loop running, which is `Active` — and none stands in for another.
///
/// # The tag is not decoration
///
/// [`AgenticLoopChunk`] is untagged and tells its own variants apart
/// by a `type` constant inside each one, while an [`Error`] is an
/// arbitrary JSON value — including, legitimately, an object with a
/// `type` field. The byte in front is what keeps a provider's error
/// text from being read as a chunk, and a failure from looking like
/// output. It is also what keeps `Active` and `Inactive` the proxy's
/// alone: a program's output is chunks, every one of them under the
/// chunk's byte, so no chunk a program writes, whatever its `type`,
/// is read as the proxy's word about the loop.
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
    /// it says so little. This is not a
    /// [`NotificationChunk`](super::NotificationChunk) with
    /// [`is_fatal`](super::NotificationChunk::is_fatal) set: that is
    /// part of the agent's OUTPUT, a loop saying it is over and why,
    /// and the scope goes on.
    Error(Error),
    /// One chunk of the agent's conversation. Tag `4`.
    ///
    /// After the id, zero or more, for as long as the scope lives, in
    /// the order the agent produced them and exactly as the proxy
    /// sent them; never before the id, never outside an
    /// [`Active`](Self::Active) and its [`Inactive`](Self::Inactive),
    /// and never after a finish. See [`AgenticLoopChunk`] for what
    /// one is.
    Chunk(AgenticLoopChunk),
    /// A loop began. Tag `5`, and nothing after it.
    ///
    /// The proxy's word that the agent's server took a message and is
    /// working: every chunk until the matching
    /// [`Inactive`](Self::Inactive) is this loop's. After the id,
    /// never twice without an `Inactive` between.
    Active,
    /// A loop ended. Tag `6`, and nothing after it.
    ///
    /// The proxy's word that the loop's stream ended — cleanly, or by
    /// dying; a loop that failed said so in a fatal notification chunk
    /// before this. The agent is quiet until the next message starts
    /// the next loop. Never without an [`Active`](Self::Active) before
    /// it.
    Inactive,
}

/// Tag for [`Frame::Id`].
const ID: u8 = 0;

/// Tag for [`Frame::VolumeHeld`].
const VOLUME_HELD: u8 = 1;

/// Tag for [`Frame::VolumeMode`].
const VOLUME_MODE: u8 = 2;

/// Tag for [`Frame::Error`].
const ERROR: u8 = 3;

/// Tag for [`Frame::Chunk`]. Public, so a relay that carries a
/// chunk's JSON without reading it can frame it.
pub const CHUNK: u8 = 4;

/// Tag for [`Frame::Active`]. Public, so a relay writes the one byte
/// that is the whole frame.
pub const ACTIVE: u8 = 5;

/// Tag for [`Frame::Inactive`]. Public, as [`ACTIVE`] is.
pub const INACTIVE: u8 = 6;

impl Encode for Frame {
    /// One failure per variant that carries JSON; the two words
    /// cannot fail.
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
            Frame::Chunk(chunk) => {
                out.extend_from_slice(&[CHUNK]);
                serde_json::to_writer(out, chunk).map_err(FrameEncodeError::Chunk)
            }
            Frame::Active => {
                out.extend_from_slice(&[ACTIVE]);
                Ok(())
            }
            Frame::Inactive => {
                out.extend_from_slice(&[INACTIVE]);
                Ok(())
            }
        }
    }
}

/// An agent container run response that could not be written.
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
                write!(f, "volume-held refusal did not serialize: {error}")
            }
            FrameEncodeError::VolumeMode(error) => {
                write!(f, "volume-mode refusal did not serialize: {error}")
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
            FrameEncodeError::VolumeMode(error) => Some(error),
            FrameEncodeError::Error(error) => Some(error),
            FrameEncodeError::Chunk(error) => Some(error),
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
            CHUNK => serde_json::from_slice(rest)
                .map(Frame::Chunk)
                .map_err(FrameError::Chunk),
            ACTIVE => Ok(Frame::Active),
            INACTIVE => Ok(Frame::Inactive),
            tag => Err(FrameError::UnknownTag(tag)),
        }
    }
}

/// An agent container run response frame that could not be read.
#[derive(Debug)]
pub enum FrameError {
    /// No bytes at all, so not even a tag.
    Empty,
    /// A tag that is none of this frame's seven.
    ///
    /// What a provider newer than its caller produces, which is the
    /// case the tag exists to make survivable: a reader that does not
    /// know a variant says so, rather than reading somebody else's
    /// bytes as its own.
    UnknownTag(u8),
    /// The id did not parse.
    Id(serde_json::Error),
    /// The held refusal did not parse.
    VolumeHeld(serde_json::Error),
    /// The mode refusal did not parse.
    VolumeMode(serde_json::Error),
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
                write!(f, "volume-held refusal did not parse: {error}")
            }
            FrameError::VolumeMode(error) => {
                write!(f, "volume-mode refusal did not parse: {error}")
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
            FrameError::VolumeMode(error) => Some(error),
            FrameError::Error(error) => Some(error),
            FrameError::Chunk(error) => Some(error),
            FrameError::Empty | FrameError::UnknownTag(_) => None,
        }
    }
}
