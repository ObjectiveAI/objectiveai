//! What a server's response frame carries.

use std::fmt;

use super::AgenticLoopChunk;
use super::{Resource, ResourceEncodeError, ResourceError};
use crate::decode::Decode;
use crate::encode::{Encode, Writer};
use crate::shared::error::Error;

/// The payload of a [`ServerFrame::Response`](crate::frame::server::ServerFrame::Response).
///
/// A server's response frames are always channel `0` — the answer to
/// the client's own request — and there are three things that answer
/// can be: a piece of the loop, the news that there will not be one,
/// or the loop's closer — the continuation, as raw bytes.
///
/// A payload leads with one byte saying which — `0` for
/// [`Chunk`](Self::Chunk), `1` for [`Error`](Self::Error), `2` for
/// [`Continuation`](Self::Continuation) — and the rest is that
/// variant's own: JSON for the first two, bytes for the third.
///
/// The tunnels do not appear here: their bytes travel the other
/// direction as [`channel_request::Frame`](crate::endpoints::agentic_loop::run::server::channel_request::Frame),
/// and what comes BACK on them is a client response, not a server one.
///
/// # The frame type is still not a wire shape
///
/// It carries no serde derives, and it is not what gets serialized —
/// it is the DISPATCH layer, saying which payload a response frame
/// holds. What changed is that the choice is now written down: it used
/// to be settled entirely by the scope the frame arrived in, and one
/// byte at the front now settles it instead.
///
/// That byte is not decoration. [`AgenticLoopChunk`] is untagged and
/// tells its own variants apart by a `type` constant inside each one,
/// while an [`Error`] is an arbitrary JSON value — including,
/// legitimately, an object with a `type` field. Leaving the two to be
/// distinguished by their JSON would mean a provider's error text
/// could be read as a chunk, and the failure would look like output.
/// And a continuation is not JSON at all: a provider's own opaque
/// state, in whatever bytes it keeps it in. The tag is what keeps
/// those bytes from ever being handed to a JSON parser.
///
/// # The closer is bytes, and it is chunked
///
/// The continuation used to be a chunk carrying a `String`, which
/// meant base64 around whatever the provider actually kept. Now it
/// is the bytes themselves, on the frame's own tag, and it CLOSES the
/// response: a provider sends it last — one frame, or several, each
/// at most
/// [`CHUNK_SIZE`](crate::CHUNK_SIZE)
/// — and nothing follows it but the finish. A run that closes with
/// no continuation frames issued none.
///
/// # The chunks are kept, not joined
///
/// A continuation is a SEQUENCE of chunks, and the boundaries are
/// part of it: the caller keeps the pieces as pieces, in order, and
/// answers the next run's
/// [`FetchContinuation`](crate::endpoints::agentic_loop::run::server::channel_request::Frame::FetchContinuation)
/// with the same pieces, one frame each, in the same order. No
/// receiver on the way joins or splits them. So a provider may put
/// meaning in the boundaries — a leading tag byte per chunk saying
/// which of several files it belongs to, say — and rely on finding
/// them exactly where it left them.
///
/// # This is not [`NotificationChunk`](super::NotificationChunk)
///
/// They are both failures and they are not the same failure.
///
/// A [`NotificationChunk`](super::NotificationChunk) with
/// [`is_fatal`](super::NotificationChunk::is_fatal) set is part of the
/// loop's OUTPUT. It arrives as a [`Chunk`](Self::Chunk) like any
/// other, carries a `_meta` bag like any other, and exists because a
/// loop can fail after producing output — ending the stream silently
/// would leave a caller unable to tell a partial result from a
/// complete one.
///
/// An [`Error`](Self::Error) is not part of the loop. It is what a
/// provider sends when there is no loop to report on, and it carries a
/// bare JSON value because this specification does not describe what
/// providers can go wrong with.
#[derive(Debug, Clone, PartialEq)]
pub enum Frame<'a> {
    /// One chunk of the answer to the client's request. Tag `0`.
    Chunk(AgenticLoopChunk),
    /// A failure. Tag `1`.
    ///
    /// See [`shared::error::Error`](crate::shared::error::Error) for
    /// why it says so little.
    Error(Error),
    /// One piece of the continuation — the run's closer. Tag `2`.
    ///
    /// Borrowed from the frame it arrived in, the fetch frames' way:
    /// a continuation is written out of a buffer and read into one,
    /// and copying every chunk in between would double each for
    /// nothing. Every frame is one chunk the caller keeps as such;
    /// the finish says the sequence is whole.
    /// A resource the run rewrote, whole, under the name of the
    /// request field that supplied it. Tag `2`. Not terminal, and
    /// may repeat — the last one wins. See [`Resource`].
    Resource(Resource<'a>),
    /// One piece of the continuation — the run's closer. Tag `3`.
    ///
    /// Borrowed from the frame it arrived in, the fetch frames' way:
    /// a continuation is written out of a buffer and read into one,
    /// and copying every chunk in between would double each for
    /// nothing. Every frame is one chunk the caller keeps as such;
    /// the finish says the sequence is whole.
    Continuation(&'a [u8]),
}

/// Tag for [`Frame::Chunk`].
const CHUNK: u8 = 0;

/// Tag for [`Frame::Error`].
const ERROR: u8 = 1;

/// Tag for [`Frame::Resource`].
const RESOURCE: u8 = 2;

/// Tag for [`Frame::Continuation`].
const CONTINUATION: u8 = 3;

/// A tag, then the variant's own payload — JSON for a chunk or an
/// error, a named body for a resource, the bytes verbatim for the
/// continuation. The newtype around a chunk still leaves no trace —
/// what a chunk encodes to is exactly what it encoded to before the
/// tag existed.
impl Encode for Frame<'_> {
    /// The JSON failure from a chunk or an error, or the resource's
    /// own. A chunk is entirely shapes: content, reasoning, tool
    /// calls, usage. Nothing in it is a passthrough.
    type Error = EncodeError;

    fn encode(&self, out: &mut Writer<'_>) -> Result<(), EncodeError> {
        match self {
            Frame::Chunk(chunk) => {
                out.extend_from_slice(&[CHUNK]);
                serde_json::to_writer(out, chunk).map_err(EncodeError::Json)
            }
            Frame::Error(error) => {
                out.extend_from_slice(&[ERROR]);
                error.encode(out).map_err(EncodeError::Json)
            }
            Frame::Resource(resource) => {
                out.extend_from_slice(&[RESOURCE]);
                resource.encode(out).map_err(EncodeError::Resource)
            }
            Frame::Continuation(bytes) => {
                out.extend_from_slice(&[CONTINUATION]);
                out.extend_from_slice(bytes);
                Ok(())
            }
        }
    }
}

impl<'a> Decode<'a> for Frame<'a> {
    /// Five ways to fail, and each names which half failed. The
    /// continuation is not among them: bytes taken as bytes cannot.
    type Error = FrameError;

    // Spelled out rather than `Self::Error`: this enum has a variant
    // called `Error`, so the associated type is ambiguous by that name.
    fn decode(bytes: &'a [u8]) -> Result<Self, FrameError> {
        let (tag, rest) = bytes.split_first().ok_or(FrameError::Empty)?;
        match *tag {
            CHUNK => serde_json::from_slice(rest)
                .map(Frame::Chunk)
                .map_err(FrameError::Chunk),
            ERROR => {
                Error::decode(rest).map(Frame::Error).map_err(FrameError::Error)
            }
            RESOURCE => Resource::decode(rest)
                .map(Frame::Resource)
                .map_err(FrameError::Resource),
            CONTINUATION => Ok(Frame::Continuation(rest)),
            tag => Err(FrameError::UnknownTag(tag)),
        }
    }
}

/// An agentic loop response frame that could not be written.
#[derive(Debug)]
pub enum EncodeError {
    /// A chunk or an error would not serialize.
    Json(serde_json::Error),
    /// The resource would not encode.
    Resource(ResourceEncodeError),
}

impl fmt::Display for EncodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EncodeError::Json(error) => {
                write!(f, "agentic loop response did not serialize: {error}")
            }
            EncodeError::Resource(error) => write!(f, "{error}"),
        }
    }
}

impl std::error::Error for EncodeError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            EncodeError::Json(error) => Some(error),
            EncodeError::Resource(error) => Some(error),
        }
    }
}

/// An agentic loop response frame that could not be read.
#[derive(Debug)]
pub enum FrameError {
    /// No bytes at all, so not even a tag.
    Empty,
    /// A tag that is none of this frame's four.
    UnknownTag(u8),
    /// The resource did not read.
    Resource(ResourceError),
    /// The chunk did not parse.
    Chunk(serde_json::Error),
    /// The error did not parse.
    ///
    /// Which is its own small joke and its own real problem: a
    /// provider whose failure report is malformed has told a caller
    /// that something went wrong and nothing else.
    Error(serde_json::Error),
}

impl fmt::Display for FrameError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FrameError::Empty => {
                f.write_str("agentic loop response frame is empty")
            }
            FrameError::UnknownTag(tag) => {
                write!(f, "unknown agentic loop response frame tag {tag}")
            }
            FrameError::Chunk(error) => {
                write!(f, "agentic loop chunk did not parse: {error}")
            }
            FrameError::Error(error) => {
                write!(f, "agentic loop error did not parse: {error}")
            }
            FrameError::Resource(error) => write!(f, "{error}"),
        }
    }
}

impl std::error::Error for FrameError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            FrameError::Chunk(error) | FrameError::Error(error) => Some(error),
            FrameError::Resource(error) => Some(error),
            FrameError::Empty | FrameError::UnknownTag(_) => None,
        }
    }
}
