//! What a container streams back.

use std::error;
use std::fmt;

use serde::{Deserialize, Serialize};

use crate::decode::Decode;
use crate::encode::{Encode, Writer};
use crate::endpoints::agentic_loop::run::server::response::{
    AgenticLoopChunk, Resource, ResourceEncodeError, ResourceError,
};

/// One frame of a container's response — one binary WebSocket
/// message, on the socket the run request opened.
///
/// A payload leads with one byte saying which, and the rest is that
/// variant's own: JSON for a chunk or a resource ask, nothing for
/// the continuation ask, bytes for the continuation itself. The
/// wire's own discipline, now on the container surface, and for the
/// wire's reason: a continuation is raw bytes, not JSON, and the tag
/// is what keeps them from ever meeting a JSON parser.
///
/// | tag | variant | payload |
/// |---|---|---|
/// | `0` | [`FetchContinuation`](Self::FetchContinuation) | nothing |
/// | `1` | [`FetchResource`](Self::FetchResource) | the ask's JSON |
/// | `2` | [`Chunk`](Self::Chunk) | the chunk's JSON |
/// | `3` | [`Resource`](Self::Resource) | name length, name, body |
/// | `4` | [`Continuation`](Self::Continuation) | the bytes, verbatim |
///
/// The order is the run's: the continuation ask opens it, resource
/// asks come as the agent needs them, chunks are the body, rewritten
/// resources surface as the run changes them, and the continuation
/// closes it.
///
/// # Who reads which
///
/// A server relaying the stream forwards tag `2` as the wire's chunk
/// frame, tag `3` as its resource frame and tag `4` as its
/// continuation frame — re-tagging one byte, never re-encoding what
/// follows it — and CONSUMES tags `0` and `1`: those are the
/// container asking the server for something, and the client never
/// sees them.
///
/// # The continuation closes the stream
///
/// The continuation's frames are the last thing a container sends:
/// one, or several, each at most
/// [`CHUNK_SIZE`](crate::endpoints::agentic_loop::run::client::channel_response::CHUNK_SIZE),
/// and then the socket closes. A run that closes without them issued
/// none. The chunks are KEPT as chunks all the way to the caller and
/// back — nobody joins or splits them — so the next run's delivery at
/// `/continuation` is the same pieces in the same order, and a
/// container may put meaning in the boundaries (a tag byte leading
/// each chunk, say).
#[derive(Debug, Clone, PartialEq)]
pub enum Response<'a> {
    /// The container asking for the continuation it resumes from.
    /// Tag `0`, and nothing after it — the tag is the whole ask;
    /// see [`FetchContinuation`]. The server's to consume.
    FetchContinuation,
    /// The container asking for a resource's bytes. Tag `1`. The
    /// server's to consume, never the client's to see. See
    /// [`FetchResource`].
    FetchResource(FetchResource),
    /// One chunk of the loop — the client's to receive, relayed
    /// verbatim. Tag `2`. See [`AgenticLoopChunk`].
    Chunk(AgenticLoopChunk),
    /// A resource the run rewrote, whole, under the name of the
    /// request field that supplied it — the client's to receive,
    /// relayed verbatim. Tag `3`. See [`Resource`].
    Resource(Resource<'a>),
    /// One piece of the run's new continuation — the closer. Tag
    /// `4`. Borrowed from the frame it is written from or read out
    /// of, the fetch frames' way: copying every chunk in between
    /// would double each for nothing. Kept as one chunk end to end.
    Continuation(&'a [u8]),
}

/// Tag for [`Response::FetchContinuation`].
const FETCH_CONTINUATION: u8 = 0;

/// Tag for [`Response::FetchResource`].
const FETCH_RESOURCE: u8 = 1;

/// Tag for [`Response::Chunk`].
const CHUNK: u8 = 2;

/// Tag for [`Response::Resource`].
const RESOURCE: u8 = 3;

/// Tag for [`Response::Continuation`].
const CONTINUATION: u8 = 4;

/// A tag, then the variant's own payload.
impl Encode for Response<'_> {
    /// The JSON failure from the two variants that carry JSON, or the
    /// resource's own; the other two cannot fail.
    type Error = ResponseEncodeError;

    fn encode(&self, out: &mut Writer<'_>) -> Result<(), ResponseEncodeError> {
        match self {
            Response::FetchContinuation => {
                out.extend_from_slice(&[FETCH_CONTINUATION]);
                Ok(())
            }
            Response::FetchResource(ask) => {
                out.extend_from_slice(&[FETCH_RESOURCE]);
                serde_json::to_writer(out, ask).map_err(ResponseEncodeError::Json)
            }
            Response::Chunk(chunk) => {
                out.extend_from_slice(&[CHUNK]);
                serde_json::to_writer(out, chunk).map_err(ResponseEncodeError::Json)
            }
            Response::Resource(resource) => {
                out.extend_from_slice(&[RESOURCE]);
                resource.encode(out).map_err(ResponseEncodeError::Resource)
            }
            Response::Continuation(bytes) => {
                out.extend_from_slice(&[CONTINUATION]);
                out.extend_from_slice(bytes);
                Ok(())
            }
        }
    }
}

impl<'a> Decode<'a> for Response<'a> {
    /// Five ways to fail, and each names which variant failed. The
    /// continuation is not among them: bytes taken as bytes cannot.
    type Error = ResponseError;

    fn decode(bytes: &'a [u8]) -> Result<Self, ResponseError> {
        let (tag, rest) = bytes.split_first().ok_or(ResponseError::Empty)?;
        match *tag {
            // Whatever follows the tag is ignored rather than
            // rejected: there is nothing this could carry, and the
            // tag already said what was meant.
            FETCH_CONTINUATION => Ok(Response::FetchContinuation),
            FETCH_RESOURCE => serde_json::from_slice(rest)
                .map(Response::FetchResource)
                .map_err(ResponseError::FetchResource),
            CHUNK => serde_json::from_slice(rest)
                .map(Response::Chunk)
                .map_err(ResponseError::Chunk),
            RESOURCE => Resource::decode(rest)
                .map(Response::Resource)
                .map_err(ResponseError::Resource),
            CONTINUATION => Ok(Response::Continuation(rest)),
            tag => Err(ResponseError::UnknownTag(tag)),
        }
    }
}

/// A container response frame that could not be written.
#[derive(Debug)]
pub enum ResponseEncodeError {
    /// A chunk or a resource ask would not serialize.
    Json(serde_json::Error),
    /// The resource would not encode.
    Resource(ResourceEncodeError),
}

impl fmt::Display for ResponseEncodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ResponseEncodeError::Json(error) => {
                write!(f, "container response did not serialize: {error}")
            }
            ResponseEncodeError::Resource(error) => write!(f, "{error}"),
        }
    }
}

impl error::Error for ResponseEncodeError {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match self {
            ResponseEncodeError::Json(error) => Some(error),
            ResponseEncodeError::Resource(error) => Some(error),
        }
    }
}

/// A container response frame that could not be read.
#[derive(Debug)]
pub enum ResponseError {
    /// No bytes at all, so not even a tag.
    Empty,
    /// A tag that is none of this frame's five.
    UnknownTag(u8),
    /// The chunk did not parse.
    Chunk(serde_json::Error),
    /// The resource ask did not parse.
    FetchResource(serde_json::Error),
    /// The resource did not read.
    Resource(ResourceError),
}

impl fmt::Display for ResponseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ResponseError::Empty => {
                f.write_str("container response frame is empty")
            }
            ResponseError::UnknownTag(tag) => {
                write!(f, "unknown container response frame tag {tag}")
            }
            ResponseError::Chunk(error) => {
                write!(f, "container chunk did not parse: {error}")
            }
            ResponseError::FetchResource(error) => {
                write!(f, "container resource ask did not parse: {error}")
            }
            ResponseError::Resource(error) => write!(f, "{error}"),
        }
    }
}

impl error::Error for ResponseError {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match self {
            ResponseError::Chunk(error) | ResponseError::FetchResource(error) => {
                Some(error)
            }
            ResponseError::Resource(error) => Some(error),
            ResponseError::Empty | ResponseError::UnknownTag(_) => None,
        }
    }
}

/// The container asking the server for a resource it only knows by
/// identity.
///
/// A request names resources as size-bearing identities — the FILE
/// grammar, `f1:<size>:<base64url sha256 of the bytes>` — and the
/// bytes live with the client, behind the server. When the
/// container needs them, this frame is the ask: the server fetches
/// the content (its own
/// [`FetchResource`](crate::endpoints::agentic_loop::run::server::channel_request::Frame::FetchResource)
/// exchange toward the client, or its own store), then POSTs the
/// bytes into the container at `/resource/{identity}` — chunks,
/// then the completion, or the error when the bytes can never
/// come; see [`resource`](super::resource).
///
/// It is not a chunk and never reaches the client: the client is
/// what the bytes come FROM.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct FetchResource {
    /// The resource's size-bearing identity, the FILE grammar:
    /// `f1:<size>:<base64url sha256 of the bytes>`.
    pub identity: String,
}

/// The continuation ask, as a frame with nothing in it —
/// [`Response::FetchContinuation`].
///
/// The container asks once, as the run starts and before it can
/// start the agent: it cannot know whether this is a first run or a
/// resume until the answer says. There is nothing to name — a run
/// resumes from the one continuation its caller holds — so the tag
/// is the whole ask. The server fetches the bytes from the client
/// over its own
/// [`FetchContinuation`](crate::endpoints::agentic_loop::run::server::channel_request::Frame::FetchContinuation)
/// exchange, then POSTs them into the container at `/continuation`
/// — chunks, then the completion (a LONE completion meaning a fresh
/// start), or the error when the bytes can never come; see
/// [`continuation`](super::continuation).
///
/// A marker type rather than nothing, so the ask has a name to be
/// documented under: the frame variant is unit, and this is its
/// doc's home.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct FetchContinuation;
