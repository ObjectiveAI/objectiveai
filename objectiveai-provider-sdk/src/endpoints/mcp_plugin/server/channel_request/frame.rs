//! What a server's channel request frame carries for an MCP plugin.

use std::error::Error;
use std::fmt;

use crate::decode::Decode;
use crate::encode::{Encode, Writer};
use crate::shared::http::request::Request;

/// One request against the caller's registry.
///
/// A payload leads with one byte and the rest is the request.
///
/// Opened only for an
/// [`ImageType::Client`](crate::shared::container::request::ImageType::Client)
/// plugin, and opened by the container RUNTIME's appetite rather than
/// the provider's: the provider serves a registry endpoint, the
/// runtime pulls from it, and every request the runtime makes that the
/// provider cannot answer from what it holds becomes one of these.
///
/// The provider understands none of it. It does not parse the manifest
/// to find layers, does not diff digests against a store of its own,
/// and does not decide what a blob is. Which means a runtime's cache is
/// the only cache, its dedup is the only dedup, and `Range` resumes and
/// `HEAD` probes work because nothing here had to be taught about them.
///
/// The repository segment of the path names the scope, so one endpoint
/// serves every creation happening at once — plugins and laboratories
/// alike — and a request routes itself without a provider keeping state
/// between them.
///
/// # A struct, and still a tag byte
///
/// A [`laboratory creation`](crate::endpoints::laboratories::create::server::channel_request::Frame)
/// asks for three things. The other two have nothing to act on here:
/// an authorization gates a connector, and a plugin has none, while a
/// write asks for content the caller never offered, since a plugin
/// takes no files. Serving an image is what is left, and it is the one
/// thing a provider needs from a caller mid-scope.
///
/// The byte stays anyway, for the same reason
/// [`write_path`](crate::shared::container::write_path::response::Frame)
/// spends one: a second thing to ask for is additive if there is a tag
/// to add to, and a wire break if there is not.
#[derive(Debug, Clone, PartialEq)]
pub struct Frame<'a>(
    /// The registry request, relayed verbatim.
    pub Request<'a>,
);

/// The tag that says this is a registry request.
const OCI: u8 = 0;

impl Encode for Frame<'_> {
    /// The ordinary JSON failure. The tag cannot fail.
    type Error = serde_json::Error;

    fn encode(&self, out: &mut Writer<'_>) -> Result<(), Self::Error> {
        out.extend_from_slice(&[OCI]);
        self.0.encode(out)
    }
}

impl<'a> Decode<'a> for Frame<'a> {
    /// Three ways to fail, and only one of them is a parse.
    type Error = FrameError;

    fn decode(bytes: &'a [u8]) -> Result<Self, Self::Error> {
        let (tag, rest) = bytes.split_first().ok_or(FrameError::Empty)?;
        if *tag != OCI {
            return Err(FrameError::UnknownTag(*tag));
        }
        Request::decode(rest).map(Frame).map_err(FrameError::Oci)
    }
}

/// An MCP plugin channel request that could not be read.
#[derive(Debug)]
pub enum FrameError {
    /// No bytes at all, so not even a tag.
    Empty,
    /// A tag this version does not define.
    ///
    /// Which is what a provider asking for something else will send,
    /// once there is something else to ask for. Until then it is a
    /// peer that disagrees about the protocol.
    UnknownTag(u8),
    /// The registry request did not parse.
    Oci(serde_json::Error),
}

impl fmt::Display for FrameError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FrameError::Empty => {
                f.write_str("mcp plugin channel request frame is empty")
            }
            FrameError::UnknownTag(tag) => {
                write!(f, "unknown mcp plugin channel request tag {tag}")
            }
            FrameError::Oci(error) => {
                write!(f, "registry request did not parse: {error}")
            }
        }
    }
}

impl Error for FrameError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            FrameError::Oci(error) => Some(error),
            FrameError::Empty | FrameError::UnknownTag(_) => None,
        }
    }
}
