//! What a server's response frame carries for an agent container begin.

use std::fmt;

use crate::wire::decode::Decode;
use crate::wire::encode::{Encode, Writer};
use crate::shared::containers::tools::Tool;
use crate::provider::endpoints::containers::agents::run::server::response::AgenticLoopChunk;
use crate::shared::error::Error;

/// A begin's answer: that the connection has begun, then the agent's
/// conversation for as long as the connection lives, each loop of it
/// bracketed by the proxy's own word that it began and that it ended
/// — or a failure.
///
/// A payload leads with one byte saying which — `0` for
/// [`Begun`](Self::Begun), `1` for [`Error`](Self::Error), `2` for
/// [`Chunk`](Self::Chunk), `3` for [`Active`](Self::Active), `4` for
/// [`Inactive`](Self::Inactive) — and the rest is that variant's own
/// JSON, or nothing: the two words carry nothing after the byte.
///
/// # Begun, then the conversation
///
/// | the scope | means |
/// |-----------|-------|
/// | a begun, then quiet, and stays open | the container holds its arguments and its tools are known, and no loop has run |
/// | a begun, an active, then chunks, and stays open | a loop is running, and the agent is speaking |
/// | …an active, chunks, an inactive, and stays open | that loop ran and ended; the agent is quiet until the next message |
/// | an error, then a finish | it has not begun — this connection had already begun, or the arguments were refused |
/// | a finish, with no error | the proxy is ending |
///
/// # The conversation is this stream
///
/// What the agent says arrives here, chunk by chunk, in the order
/// the agent's server streams them, and the server carries each one
/// on to the caller as the run scope's own
/// [`Chunk`](crate::provider::endpoints::containers::agents::run::server::response::Frame::Chunk). Nothing opens a
/// loop: an [`Enqueue`](super::super::super::client::channel_request::Frame::Enqueue)
/// with no loop running starts one on its message, and one while a
/// loop runs joins the queue. The tools family's stream carries
/// nothing after `Begun`.
///
/// # The proxy marks the loop, and only the proxy can
///
/// A loop is one `POST /run` on the agent's server: it begins when
/// that call answers `2xx`, and it ends when the event stream of that
/// answer ends, however it ends. The proxy makes the call, so the
/// proxy is the one party that knows both moments, and it says them
/// here as [`Active`](Self::Active) before the loop's first chunk and
/// [`Inactive`](Self::Inactive) after its last. The program's stream
/// is chunks and only chunks, every one of them a `Chunk` here, so
/// nothing a program writes is read as either word: see
/// [the run's frame](crate::provider::endpoints::containers::agents::run::server::response::Frame)
/// for why the byte in front is what keeps that so. Between an
/// `Inactive` and the next `Active` the agent says nothing; quiet
/// between an `Active` and its `Inactive` is an agent still working.
///
/// The asks the container makes, and the family's other exchanges,
/// are channels, not this stream. It carries no readiness signal
/// beyond `Begun`: the proxy is here, the arguments are held, and
/// channels may be opened.
#[derive(Debug, Clone, PartialEq)]
pub enum Frame {
    /// The connection has begun, the container holds its arguments
    /// for its life, and these are the tools it declared. Tag `0`.
    ///
    /// Arrives once, when the container's server has taken the
    /// arguments and answered with its tools — as JSON, a list, empty
    /// for a program that needs none; see
    /// [`tools`](crate::shared::containers::tools) for what one is
    /// and what the provider does with the list. A channel on this
    /// scope is opened only after it.
    Begun(Vec<Tool>),
    /// A failure. Tag `1`.
    ///
    /// A second begin on a connection that had one, or arguments the
    /// container refused — the container's server's own words: a
    /// value the image will not take. It is the one variant that ends the
    /// scope rather than adding to it. See
    /// [`shared::error::Error`](crate::shared::error::Error) for why
    /// it says so little.
    Error(Error),
    /// One chunk of the agent's conversation. Tag `2`.
    ///
    /// After `Begun`, zero or more, for as long as the connection
    /// lives, each as the agent's server streamed it — the proxy sets
    /// nothing under its `_meta`; the daemon that keeps it does, see
    /// [`shared::mcp`](crate::shared::mcp); never before
    /// `Begun`, never outside an [`Active`](Self::Active) and its
    /// [`Inactive`](Self::Inactive), and never after a finish. See
    /// [`AgenticLoopChunk`] for what one is.
    Chunk(AgenticLoopChunk),
    /// A loop began. Tag `3`, and nothing after it.
    ///
    /// The agent's server answered `POST /run` with `2xx`: a loop is
    /// running, and the chunks that follow until the matching
    /// [`Inactive`](Self::Inactive) are its output. Sent by the proxy,
    /// never relayed from the program.
    Active,
    /// A loop ended. Tag `4`, and nothing after it.
    ///
    /// The event stream of the loop's `POST /run` ended — cleanly, or
    /// by dying; a loop that failed said so in a fatal notification
    /// chunk before this. The agent is quiet until the next message
    /// starts the next loop. Sent by the proxy, never relayed from
    /// the program.
    Inactive,
}

/// Tag for [`Frame::Begun`].
const BEGUN: u8 = 0;

/// Tag for [`Frame::Error`].
const ERROR: u8 = 1;

/// Tag for [`Frame::Chunk`]. Public, so a relay that carries a
/// chunk's JSON without reading it can frame it.
pub const CHUNK: u8 = 2;

/// Tag for [`Frame::Active`]. Public, so the proxy writes the one
/// byte that is the whole frame.
pub const ACTIVE: u8 = 3;

/// Tag for [`Frame::Inactive`]. Public, as [`ACTIVE`] is.
pub const INACTIVE: u8 = 4;

/// A tag, and that variant's own JSON — or the tag alone.
impl Encode for Frame {
    /// The ordinary JSON failure.
    type Error = serde_json::Error;

    // Spelled out rather than `Self::Error`: this enum has a variant
    // called `Error`, so the associated type is ambiguous by that name.
    fn encode(
        &self,
        out: &mut Writer<'_>,
    ) -> Result<(), serde_json::Error> {
        match self {
            Frame::Begun(tools) => {
                out.extend_from_slice(&[BEGUN]);
                serde_json::to_writer(out, tools)
            }
            Frame::Error(error) => {
                out.extend_from_slice(&[ERROR]);
                error.encode(out)
            }
            Frame::Chunk(chunk) => {
                out.extend_from_slice(&[CHUNK]);
                serde_json::to_writer(out, chunk)
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

impl Decode<'_> for Frame {
    /// Five ways to fail, and three of them are JSON.
    type Error = FrameError;

    // Spelled out for the same reason as `encode` above.
    fn decode(bytes: &[u8]) -> Result<Self, FrameError> {
        let (tag, rest) = bytes.split_first().ok_or(FrameError::Empty)?;
        match *tag {
            BEGUN => serde_json::from_slice(rest).map(Frame::Begun).map_err(FrameError::Begun),
            ERROR => {
                Error::decode(rest).map(Frame::Error).map_err(FrameError::Error)
            }
            CHUNK => serde_json::from_slice(rest)
                .map(Frame::Chunk)
                .map_err(FrameError::Chunk),
            ACTIVE => Ok(Frame::Active),
            INACTIVE => Ok(Frame::Inactive),
            tag => Err(FrameError::UnknownTag(tag)),
        }
    }
}

/// An agents begin response frame that could not be read.
#[derive(Debug)]
pub enum FrameError {
    /// No bytes at all, so not even a tag.
    Empty,
    /// A tag that is none of this frame's five.
    ///
    /// What a proxy newer than its server produces, which is the case
    /// the tag exists to make survivable: a reader that does not know
    /// a variant says so, rather than reading somebody else's bytes as
    /// its own.
    UnknownTag(u8),
    /// The tools did not parse.
    Begun(serde_json::Error),
    /// The error did not parse.
    Error(serde_json::Error),
    /// The chunk did not parse.
    Chunk(serde_json::Error),
}

impl fmt::Display for FrameError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FrameError::Empty => {
                f.write_str("agents begin response frame is empty")
            }
            FrameError::UnknownTag(tag) => {
                write!(f, "unknown agents begin response frame tag {tag}")
            }
            FrameError::Begun(error) => {
                write!(f, "agents begin tools did not parse: {error}")
            }
            FrameError::Error(error) => {
                write!(f, "agents begin error did not parse: {error}")
            }
            FrameError::Chunk(error) => {
                write!(f, "agents begin chunk did not parse: {error}")
            }
        }
    }
}

impl std::error::Error for FrameError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            FrameError::Begun(error) | FrameError::Error(error) | FrameError::Chunk(error) => Some(error),
            FrameError::Empty | FrameError::UnknownTag(_) => None,
        }
    }
}
