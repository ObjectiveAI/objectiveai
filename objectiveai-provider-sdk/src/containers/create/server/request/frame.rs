//! What a server's channel request frame carries for a creation.

use std::error::Error;
use std::fmt;

use crate::decode::Decode;
use crate::encode::{Encode, Writer};
use crate::http::request::Request;

/// One request against the caller's registry.
///
/// Opened only for an
/// [`ImageType::Client`](crate::containers::create::client::request::ImageType::Client)
/// creation, and opened by the container RUNTIME's appetite rather
/// than the provider's: the provider serves a registry endpoint, the
/// runtime pulls from it, and every request the runtime makes that the
/// provider cannot answer from what it already holds becomes one of
/// these.
///
/// # The provider understands none of it
///
/// It does not parse the manifest to find layers, does not diff
/// digests against a store of its own, and does not decide what a blob
/// is. It relays. Which means a container runtime's cache is the only
/// cache, its dedup is the only dedup, and `Range` resumes and `HEAD`
/// probes work because nothing here had to be taught about them.
///
/// The repository segment of the path names the scope, so one endpoint
/// serves every creation happening at once and a request routes itself
/// without a provider keeping state between them.
#[derive(Debug, Clone, PartialEq)]
pub struct Frame<'a>(
    /// The request, verbatim.
    pub Request<'a>,
);

/// This frame's tag among a creation's channel requests.
///
/// The only one so far. Carried anyway, so a second kind of ask is a
/// new tag rather than a new frame type.
const TAG: u8 = 0;

impl Encode for Frame<'_> {
    /// The ordinary JSON failure. The tag cannot fail.
    type Error = serde_json::Error;

    fn encode(&self, out: &mut Writer<'_>) -> Result<(), Self::Error> {
        out.extend_from_slice(&[TAG]);
        self.0.encode(out)
    }
}

impl<'a> Decode<'a> for Frame<'a> {
    /// Three ways to fail, and only one of them is JSON.
    type Error = FrameError;

    fn decode(bytes: &'a [u8]) -> Result<Self, Self::Error> {
        let (tag, rest) = bytes.split_first().ok_or(FrameError::Empty)?;
        if *tag != TAG {
            return Err(FrameError::UnexpectedTag(*tag));
        }
        Request::decode(rest).map(Frame).map_err(FrameError::Body)
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

impl fmt::Display for FrameError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FrameError::Empty => {
                f.write_str("creation channel request frame is empty")
            }
            FrameError::UnexpectedTag(tag) => {
                write!(
                    f,
                    "expected creation channel request tag {TAG},                      found {tag}"
                )
            }
            FrameError::Body(error) => {
                write!(f, "creation channel request did not parse: {error}")
            }
        }
    }
}

impl Error for FrameError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            FrameError::Body(error) => Some(error),
            FrameError::Empty | FrameError::UnexpectedTag(_) => None,
        }
    }
}
