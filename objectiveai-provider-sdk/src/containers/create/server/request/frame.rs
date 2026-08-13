//! What a server's channel request frame carries for a creation.

use serde::{Deserialize, Serialize};

use crate::decode::Decode;
use crate::encode::{Encode, Writer};

/// Send the content with this digest.
///
/// Opened only against an
/// [`Image::Client`](crate::containers::create::client::request::Image::Client)
/// creation, and opened by the container RUNTIME's appetite rather
/// than the provider's: the provider serves a registry endpoint, the
/// runtime pulls from it, and a request the runtime makes for content
/// it does not already hold becomes one of these.
///
/// Which is why nothing here says what the content IS. A layer, the
/// config, a child manifest of a multi-platform index — content
/// addressing does not distinguish them, and neither does this. The
/// caller looks up bytes by digest and sends them.
///
/// # No name, and no scope
///
/// Both are already known. A creation names one image, so the scope
/// this arrives in is the only repository there is; and the digest is
/// the whole of the question, because it is the whole of the answer's
/// identity.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub struct Frame {
    /// The digest, `<algorithm>:<hex>`.
    ///
    /// From the manifest the caller sent. A provider does not invent
    /// these — every one it asks for appeared in a descriptor the
    /// caller wrote.
    pub digest: String,
}

/// This frame's tag among a creation's channel requests.
///
/// The only one so far. Carried anyway, so a second kind of ask is a
/// new tag rather than a new frame type — the same reason an empty
/// request still spends a byte.
const TAG: u8 = 0;

impl Encode for Frame {
    /// The ordinary JSON failure. The tag cannot fail.
    type Error = serde_json::Error;

    fn encode(&self, out: &mut Writer<'_>) -> Result<(), Self::Error> {
        out.extend_from_slice(&[TAG]);
        serde_json::to_writer(out, self)
    }
}

impl Decode<'_> for Frame {
    /// Three ways to fail, and only one of them is JSON.
    type Error = FrameError;

    fn decode(bytes: &[u8]) -> Result<Self, Self::Error> {
        let (tag, rest) = bytes.split_first().ok_or(FrameError::Empty)?;
        if *tag != TAG {
            return Err(FrameError::UnexpectedTag(*tag));
        }
        serde_json::from_slice(rest).map_err(FrameError::Body)
    }
}

/// A creation channel request that could not be read.
#[derive(Debug)]
pub enum FrameError {
    /// No bytes at all, so not even a tag.
    Empty,
    /// A tag naming some other request.
    UnexpectedTag(u8),
    /// The request did not parse.
    Body(serde_json::Error),
}

impl std::fmt::Display for FrameError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FrameError::Empty => {
                f.write_str("creation channel request frame is empty")
            }
            FrameError::UnexpectedTag(tag) => {
                write!(
                    f,
                    "expected creation channel request tag {TAG}, \
                     found {tag}"
                )
            }
            FrameError::Body(error) => {
                write!(f, "creation channel request did not parse: {error}")
            }
        }
    }
}

impl std::error::Error for FrameError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            FrameError::Body(error) => Some(error),
            FrameError::Empty | FrameError::UnexpectedTag(_) => None,
        }
    }
}
