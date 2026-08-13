//! What a client's response frame carries during a creation.

use crate::decode::Decode;
use crate::encode::{Encode, Writer};

/// The content a provider asked for, or the admission that it is not
/// there.
///
/// # Bytes, and almost nothing else
///
/// A registry blob response needs a content type, a length and a
/// digest header, and a provider synthesizes all three without asking:
/// every descriptor in the manifest the caller already sent carries
/// `mediaType`, `size` and `digest`. So there is no head to send here
/// the way an MCP response has one. The caller contributes the bytes
/// and the provider contributes everything that describes them.
///
/// [`Body`](Self::Body) frames stream — a blob is a layer and layers
/// are large, so a caller sends as many as it likes and the channel's
/// finish ends the content.
///
/// # Why `Unavailable` exists
///
/// Because an empty blob is real. The empty gzip layer is a genuine,
/// commonly-occurring blob with a genuine digest, so a channel that
/// carried no bytes and then finished would be indistinguishable from
/// one that succeeded in sending nothing.
///
/// That leaves a provider unable to tell "here is your zero-byte
/// layer" from "I cannot produce this", and one of those is a
/// creation that should fail. One tag value settles it.
///
/// A caller should not need this. A provider only ever asks for
/// digests that appeared in a manifest the caller itself wrote, so a
/// caller that cannot produce one has contradicted itself. It is here
/// for that case to be reportable rather than a silence a provider
/// has to wait out.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Frame<'a> {
    /// A piece of the content. Tag `0`.
    Body(&'a [u8]),
    /// The caller does not have it. Tag `1`, and nothing follows.
    Unavailable,
}

/// Tag for [`Frame::Body`].
const BODY: u8 = 0;

/// Tag for [`Frame::Unavailable`].
const UNAVAILABLE: u8 = 1;

impl Encode for Frame<'_> {
    /// [`Infallible`](std::convert::Infallible): a tag and a slice
    /// copy, neither of which can fail.
    type Error = std::convert::Infallible;

    fn encode(&self, out: &mut Writer<'_>) -> Result<(), Self::Error> {
        match self {
            Frame::Body(body) => {
                out.extend_from_slice(&[BODY]);
                out.extend_from_slice(body);
            }
            Frame::Unavailable => out.extend_from_slice(&[UNAVAILABLE]),
        }
        Ok(())
    }
}

impl<'a> Decode<'a> for Frame<'a> {
    /// Two ways to fail, and neither is a parse.
    type Error = FrameError;

    fn decode(bytes: &'a [u8]) -> Result<Self, Self::Error> {
        let (tag, rest) = bytes.split_first().ok_or(FrameError::Empty)?;
        match *tag {
            BODY => Ok(Frame::Body(rest)),
            UNAVAILABLE => Ok(Frame::Unavailable),
            tag => Err(FrameError::UnknownTag(tag)),
        }
    }
}

/// A creation response frame that could not be read.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FrameError {
    /// No bytes at all, so not even a tag.
    ///
    /// Distinct from a zero-length body, which is a tag followed by
    /// nothing and is the very case [`Frame::Unavailable`] exists to
    /// stay separate from.
    Empty,
    /// A tag that is neither [`Frame::Body`] nor
    /// [`Frame::Unavailable`].
    UnknownTag(u8),
}

impl std::fmt::Display for FrameError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FrameError::Empty => {
                f.write_str("creation response frame is empty")
            }
            FrameError::UnknownTag(tag) => {
                write!(f, "unknown creation response frame tag {tag}")
            }
        }
    }
}

impl std::error::Error for FrameError {}
