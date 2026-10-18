//! What a server's response frame carries for an agent container begin.

use std::fmt;

use super::Registered;
use crate::wire::decode::Decode;
use crate::wire::encode::{Encode, Writer};
use crate::provider::endpoints::containers::agents::run::server::response::AgenticLoopChunk;
use crate::shared::containers::secrets::Values;
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
/// JSON, or nothing: the one bare word carries nothing after the
/// byte.
///
/// # Begun, then the conversation
///
/// | the scope | means |
/// |-----------|-------|
/// | a begun, then quiet, and stays open | the container holds its arguments, what its program declared is known, and no loop has run |
/// | a begun, an active, then chunks, and stays open | a loop is running, and the agent is speaking |
/// | …an active, chunks, an inactive, and stays open | that loop ran and ended, its rotating secrets collected; the agent is quiet until the next message |
/// | an error, then a finish | it has not begun — this connection had already begun, or the arguments were refused; or, after `Begun`, a loop's secrets could not be collected |
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
/// loop runs joins the queue. The tools family's stream carries the
/// two words and nothing else after `Begun`.
///
/// # The proxy marks the loop, and only the proxy can
///
/// A loop is one `POST /run` on the agent's server: it begins when
/// that call answers `2xx`, and it ends when the event stream of that
/// answer ends, however it ends. The proxy makes the call, so the
/// proxy is the one party that knows both moments, and it says them
/// here as [`Active`](Self::Active) before the loop's first chunk and
/// [`Inactive`](Self::Inactive) after its last — the latter once the
/// program has handed back its rotating secrets, which ride it. The
/// program's stream is chunks and only chunks, every one of them a
/// `Chunk` here, so nothing a program writes is read as either word:
/// see
/// [the run's frame](crate::provider::endpoints::containers::agents::run::server::response::Frame)
/// for why the byte in front is what keeps that so. Between an
/// `Inactive` and the next `Active` the agent says nothing; quiet
/// between an `Active` and its `Inactive` is an agent still working.
///
/// # Nothing is delivered before a feed
///
/// After `Begun`, and after every `Inactive`, the proxy delivers
/// nothing to a program that declared a secret until the server has
/// fed the values on a
/// [`Secrets`](super::super::super::client::channel_request::Frame::Secrets)
/// channel and the proxy has answered it `Fed`: a message enqueued
/// meanwhile waits. A program that declared none is never gated.
///
/// The asks the container makes, and the family's other exchanges,
/// are channels, not this stream. It carries no readiness signal
/// beyond `Begun`: the proxy is here, the arguments are held, and
/// channels may be opened.
#[derive(Debug, Clone, PartialEq)]
pub enum Frame {
    /// The connection has begun, the container holds its arguments
    /// for its life, and this is what its program declared. Tag `0`.
    ///
    /// Arrives once, when the container's server has taken the
    /// arguments and answered its registration — as JSON, a
    /// [`Registered`]: the dependencies, a list, empty for a program
    /// that needs none, and the secrets it takes, a map, empty for a
    /// program that takes none; see
    /// [`registration`](crate::shared::containers::registration) for
    /// what the provider does with both. A channel on this scope is
    /// opened only after it.
    Begun(Registered),
    /// A failure. Tag `1`.
    ///
    /// A second begin on a connection that had one, or arguments the
    /// container refused — the container's server's own words: a
    /// value the image will not take — or, after `Begun`, a loop's
    /// rotating secrets the program would not hand back, named and
    /// never valued. It is the one variant that ends the scope rather
    /// than adding to it. See
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
    /// A loop ended, and these are its rotating secrets. Tag `4`.
    ///
    /// The event stream of the loop's `POST /run` ended — cleanly, or
    /// by dying; a loop that failed said so in a fatal notification
    /// chunk before this — and the program handed back every
    /// `rotating` secret it declared, each by name to the value it
    /// leaves in it, as JSON after the byte: the [`Values`] the
    /// caller writes back. Empty for a program that declared none
    /// rotating. The agent is quiet until the next message starts the
    /// next loop. Sent by the proxy, never relayed from the program.
    Inactive(Values),
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

/// Tag for [`Frame::Inactive`]. Public, so a relay that carries the
/// values' JSON without reading it can frame it.
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
            Frame::Begun(registered) => {
                out.extend_from_slice(&[BEGUN]);
                serde_json::to_writer(out, registered)
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
            Frame::Inactive(values) => {
                out.extend_from_slice(&[INACTIVE]);
                serde_json::to_writer(out, values)
            }
        }
    }
}

impl Decode<'_> for Frame {
    /// Six ways to fail, and four of them are JSON.
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
            INACTIVE => serde_json::from_slice(rest)
                .map(Frame::Inactive)
                .map_err(FrameError::Inactive),
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
    /// What the program declared did not parse.
    Begun(serde_json::Error),
    /// The error did not parse.
    Error(serde_json::Error),
    /// The chunk did not parse.
    Chunk(serde_json::Error),
    /// The rotating secrets did not parse. The message is the
    /// parser's, which quotes no value.
    Inactive(serde_json::Error),
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
                write!(f, "agents begin registration did not parse: {error}")
            }
            FrameError::Error(error) => {
                write!(f, "agents begin error did not parse: {error}")
            }
            FrameError::Chunk(error) => {
                write!(f, "agents begin chunk did not parse: {error}")
            }
            FrameError::Inactive(error) => {
                write!(f, "agents begin inactive secrets did not parse: {error}")
            }
        }
    }
}

impl std::error::Error for FrameError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            FrameError::Begun(error)
            | FrameError::Error(error)
            | FrameError::Chunk(error)
            | FrameError::Inactive(error) => Some(error),
            FrameError::Empty | FrameError::UnknownTag(_) => None,
        }
    }
}
