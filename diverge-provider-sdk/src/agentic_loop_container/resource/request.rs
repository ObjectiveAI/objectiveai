//! One delivery — a chunk of a resource, or the word that it is
//! whole.

use std::convert::Infallible;
use std::error;
use std::fmt;

use crate::decode::Decode;
use crate::encode::{Encode, Writer};

/// The body of one `POST /resource/{identity}`.
///
/// The identity rides the PATH, not the body: the route names WHICH
/// resource, and the body says only what happened to it — one tag
/// byte, then the chunk's bytes when the tag says chunk. Binary,
/// because the payload is bytes and base64 is not welcome.
///
/// The body is borrowed from the request it arrived in, the
/// [`oci`](crate::shared::oci::response::Frame) way: the receiver
/// is about to append these bytes somewhere, and copying them first
/// would double every chunk's memory for nothing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Request<'a> {
    /// One chunk of the resource's bytes. Tag `0`, then the bytes,
    /// verbatim.
    ///
    /// Appended onto what arrived before for the path's identity —
    /// chunk-naive by design: same identity, next POST, append. The
    /// sender splits at
    /// [`CHUNK_SIZE`](crate::endpoints::agentic_loop::run::client::channel_response::CHUNK_SIZE)
    /// and only splits what exceeds it, so an empty chunk does not
    /// occur; the empty resource is a lone
    /// [`Complete`](Self::Complete).
    Chunk {
        /// This chunk of the bytes, borrowed from the request they
        /// arrived in.
        body: &'a [u8],
    },
    /// Every chunk is in. Tag `1`, and nothing after it.
    ///
    /// What lets the container act on the resource: until this
    /// arrives, more bytes may follow. Whether the whole is right
    /// is the identity's promise — the size and hash it carries are
    /// exactly what a short or wrong delivery fails.
    Complete,
}

/// Tag for [`Request::Chunk`].
const CHUNK: u8 = 0;

/// Tag for [`Request::Complete`].
const COMPLETE: u8 = 1;

/// A tag, then whatever the variant carries.
impl Encode for Request<'_> {
    /// Bytes copied to bytes: nothing to fail.
    type Error = Infallible;

    fn encode(&self, out: &mut Writer<'_>) -> Result<(), Infallible> {
        match self {
            Request::Chunk { body } => {
                out.extend_from_slice(&[CHUNK]);
                out.extend_from_slice(body);
            }
            Request::Complete => {
                out.extend_from_slice(&[COMPLETE]);
            }
        }
        Ok(())
    }
}

impl<'a> Decode<'a> for Request<'a> {
    /// Three ways to fail, none of them JSON.
    type Error = FrameError;

    fn decode(bytes: &'a [u8]) -> Result<Self, FrameError> {
        let (tag, rest) = bytes.split_first().ok_or(FrameError::Empty)?;
        match *tag {
            CHUNK => Ok(Request::Chunk { body: rest }),
            COMPLETE => {
                if rest.is_empty() {
                    Ok(Request::Complete)
                } else {
                    Err(FrameError::Trailing(rest.len()))
                }
            }
            tag => Err(FrameError::UnknownTag(tag)),
        }
    }
}

/// A delivery that could not be read.
#[derive(Debug)]
pub enum FrameError {
    /// No bytes at all, so not even a tag.
    Empty,
    /// A tag that is neither of this body's two.
    UnknownTag(u8),
    /// Bytes after a completion's tag, where nothing belongs.
    Trailing(usize),
}

impl fmt::Display for FrameError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FrameError::Empty => f.write_str("resource body is empty"),
            FrameError::UnknownTag(tag) => {
                write!(f, "unknown resource tag {tag}")
            }
            FrameError::Trailing(len) => write!(
                f,
                "resource completion carries {len} bytes after the tag"
            ),
        }
    }
}

impl error::Error for FrameError {}
