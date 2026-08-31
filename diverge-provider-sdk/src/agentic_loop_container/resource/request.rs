//! One delivery — a chunk of a resource, or the word that it is
//! whole.

use std::error;
use std::fmt;
use std::str::Utf8Error;

use crate::decode::Decode;
use crate::encode::{Encode, Writer};

/// The body of one `POST /resource`.
///
/// Binary, led by a tag byte — the payload is bytes and base64 is
/// not welcome — with the identity in front of them so a container
/// receiving several resources knows which one grew.
///
/// The body is borrowed from the request it arrived in, the
/// [`oci`](crate::shared::oci::response::Frame) way: the receiver
/// is about to append these bytes somewhere, and copying them first
/// would double every chunk's memory for nothing.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Request<'a> {
    /// One chunk of the resource's bytes. Tag `0`, then
    /// `[u32 BE: identity byte length][identity, UTF-8][the bytes,
    /// verbatim]`.
    ///
    /// Appended onto what arrived before for the same identity —
    /// chunk-naive by design: same identity, next POST, append. The
    /// sender splits at
    /// [`CHUNK_SIZE`](crate::endpoints::agentic_loop::run::client::channel_response::CHUNK_SIZE)
    /// and only splits what exceeds it, so an empty chunk does not
    /// occur; the empty resource is a lone
    /// [`Complete`](Self::Complete).
    Chunk {
        /// The resource's size-bearing identity, the FILE grammar:
        /// `f1:<size>:<base64url sha256 of the bytes>`.
        identity: String,
        /// This chunk of the bytes, borrowed from the request they
        /// arrived in.
        body: &'a [u8],
    },
    /// Every chunk is in. Tag `1`, then the identity as above, and
    /// nothing after it.
    ///
    /// What lets the container act on the resource: until this
    /// arrives, more bytes may follow. Whether the whole is right
    /// is the identity's promise — the size and hash it carries are
    /// exactly what a short or wrong delivery fails.
    Complete {
        /// The resource's size-bearing identity.
        identity: String,
    },
}

/// Tag for [`Request::Chunk`].
const CHUNK: u8 = 0;

/// Tag for [`Request::Complete`].
const COMPLETE: u8 = 1;

/// The bytes the identity length occupies.
const IDENTITY_LEN: usize = 4;

/// A tag, a length-prefixed identity, then whatever the variant
/// carries.
impl Encode for Request<'_> {
    /// The one thing that can fail: an identity longer than the
    /// four-byte length can say.
    type Error = EncodeError;

    fn encode(&self, out: &mut Writer<'_>) -> Result<(), EncodeError> {
        let (tag, identity, body) = match self {
            Request::Chunk { identity, body } => {
                (CHUNK, identity, *body)
            }
            Request::Complete { identity } => {
                (COMPLETE, identity, &[] as &[u8])
            }
        };
        let len = u32::try_from(identity.len())
            .map_err(|_| EncodeError::IdentityLength(identity.len()))?;
        out.extend_from_slice(&[tag]);
        out.extend_from_slice(&len.to_be_bytes());
        out.extend_from_slice(identity.as_bytes());
        out.extend_from_slice(body);
        Ok(())
    }
}

impl<'a> Decode<'a> for Request<'a> {
    /// Five ways to fail, none of them JSON.
    type Error = FrameError;

    fn decode(bytes: &'a [u8]) -> Result<Self, FrameError> {
        let (tag, rest) = bytes.split_first().ok_or(FrameError::Empty)?;
        if rest.len() < IDENTITY_LEN {
            return Err(FrameError::Short(rest.len()));
        }
        let (len, rest) = rest.split_at(IDENTITY_LEN);
        let len = u32::from_be_bytes(
            <[u8; IDENTITY_LEN]>::try_from(len)
                .expect("split_at gave 4 bytes"),
        ) as usize;
        if rest.len() < len {
            return Err(FrameError::Truncated {
                need: len,
                have: rest.len(),
            });
        }
        let (identity, body) = rest.split_at(len);
        let identity = std::str::from_utf8(identity)
            .map_err(FrameError::Identity)?
            .to_string();
        match *tag {
            CHUNK => Ok(Request::Chunk { identity, body }),
            COMPLETE => {
                if body.is_empty() {
                    Ok(Request::Complete { identity })
                } else {
                    Err(FrameError::Trailing(body.len()))
                }
            }
            tag => Err(FrameError::UnknownTag(tag)),
        }
    }
}

/// A delivery that could not be written.
#[derive(Debug)]
pub enum EncodeError {
    /// An identity longer than the four-byte length can say.
    /// Carried rather than truncated, because a length that lies is
    /// a body that reads somebody else's bytes as its own.
    IdentityLength(usize),
}

impl fmt::Display for EncodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EncodeError::IdentityLength(len) => write!(
                f,
                "resource identity is {len} bytes, more than a u32 can say"
            ),
        }
    }
}

impl error::Error for EncodeError {}

/// A delivery that could not be read.
#[derive(Debug)]
pub enum FrameError {
    /// No bytes at all, so not even a tag.
    Empty,
    /// A tag that is neither of this body's two.
    UnknownTag(u8),
    /// Fewer bytes after the tag than the identity length itself,
    /// carrying however many there were.
    Short(usize),
    /// An identity length pointing past the end of the body.
    Truncated {
        /// What the length claimed.
        need: usize,
        /// What was actually there.
        have: usize,
    },
    /// An identity that is not UTF-8.
    Identity(Utf8Error),
    /// Bytes after a completion's identity, where nothing belongs.
    Trailing(usize),
}

impl fmt::Display for FrameError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FrameError::Empty => {
                f.write_str("resource body is empty")
            }
            FrameError::UnknownTag(tag) => {
                write!(f, "unknown resource tag {tag}")
            }
            FrameError::Short(len) => write!(
                f,
                "resource body is {len} bytes after the tag, not even an \
                 identity length"
            ),
            FrameError::Truncated { need, have } => write!(
                f,
                "resource identity claims {need} bytes and {have} follow"
            ),
            FrameError::Identity(error) => {
                write!(f, "resource identity is not UTF-8: {error}")
            }
            FrameError::Trailing(len) => write!(
                f,
                "resource completion carries {len} bytes after the identity"
            ),
        }
    }
}

impl error::Error for FrameError {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match self {
            FrameError::Identity(error) => Some(error),
            FrameError::Empty
            | FrameError::UnknownTag(_)
            | FrameError::Short(_)
            | FrameError::Truncated { .. }
            | FrameError::Trailing(_) => None,
        }
    }
}
