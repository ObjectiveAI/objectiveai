//! What a server's channel request frame carries for a creation.

use std::error::Error;
use std::fmt;

use crate::decode::Decode;
use crate::encode::{Encode, Writer};
use crate::http::request::Request;

/// What a provider asks a caller for while a creation runs.
///
/// A payload leads with one byte saying which — `0` for
/// [`Oci`](Self::Oci), `1` for [`Authorize`](Self::Authorize) — and
/// the rest is that variant's own bytes. The frame's own `type` could
/// have carried this and deliberately does not: a frame already
/// carries one payload's worth of protocol, and splitting the
/// discrimination across the envelope and the payload would mean two
/// vocabularies to version and two places to keep in step.
#[derive(Debug, Clone, PartialEq)]
pub enum Frame<'a> {
    /// One request against the caller's registry.
    ///
    /// Opened only for an
    /// [`ImageType::Client`](crate::containers::create::client::request::ImageType::Client)
    /// creation, and opened by the container RUNTIME's appetite rather
    /// than the provider's: the provider serves a registry endpoint,
    /// the runtime pulls from it, and every request the runtime makes
    /// that the provider cannot answer from what it holds becomes one
    /// of these.
    ///
    /// The provider understands none of it. It does not parse the
    /// manifest to find layers, does not diff digests against a store
    /// of its own, and does not decide what a blob is. Which means a
    /// runtime's cache is the only cache, its dedup is the only dedup,
    /// and `Range` resumes and `HEAD` probes work because nothing here
    /// had to be taught about them.
    ///
    /// The repository segment of the path names the scope, so one
    /// endpoint serves every creation happening at once and a request
    /// routes itself without a provider keeping state between them.
    Oci(Request<'a>),
    /// Ask the caller whether a connector may attach to the
    /// container.
    ///
    /// Opened when one arrives. A yes is what
    /// [`Connections`](crate::containers::create::server::response::Frame::Connections)
    /// then reflects; a no is a connector that never joins.
    ///
    /// The payload is arbitrary — whatever identifies a connector, and
    /// whatever a caller needs in order to decide, are for the two ends
    /// to agree. This layer guarantees only that the question is asked
    /// on its own channel and answered before anything depends on the
    /// answer.
    ///
    /// # It has a reply, where connection auth does not
    ///
    /// [`Auth`](crate::frame::server::ServerFrame::Auth) is the same
    /// shape and gets no response at all: a credential that is
    /// accepted is followed by the connection working, one that is not
    /// by a close, and a peer that has not authenticated cannot make
    /// the far end compose anything. That is deliberate, and it costs
    /// diagnosis to avoid being an oracle.
    ///
    /// None of that applies here. The peer is already authenticated
    /// and already inside a scope it opened, so there is no
    /// amplification to deny it and no secret a yes-or-no could leak
    /// that it does not already have. A plain answer is safe, so it
    /// gets one:
    /// [`authorize::Frame`](crate::containers::create::client::channel_response::authorize::Frame).
    Authorize(&'a [u8]),
}

/// Tag for [`Frame::Oci`].
const OCI: u8 = 0;

/// Tag for [`Frame::Authorize`].
const AUTHORIZE: u8 = 1;

impl Encode for Frame<'_> {
    /// Only [`Oci`](Frame::Oci) can fail, and only the way any JSON
    /// serialization can. An authorization payload is bytes and has
    /// nothing to get wrong.
    type Error = serde_json::Error;

    fn encode(&self, out: &mut Writer<'_>) -> Result<(), Self::Error> {
        match self {
            Frame::Oci(request) => {
                out.extend_from_slice(&[OCI]);
                request.encode(out)
            }
            Frame::Authorize(payload) => {
                out.extend_from_slice(&[AUTHORIZE]);
                out.extend_from_slice(payload);
                Ok(())
            }
        }
    }
}

impl<'a> Decode<'a> for Frame<'a> {
    /// Three ways to fail, and only one of them is JSON.
    type Error = FrameError;

    fn decode(bytes: &'a [u8]) -> Result<Self, Self::Error> {
        let (tag, rest) = bytes.split_first().ok_or(FrameError::Empty)?;
        match *tag {
            OCI => Request::decode(rest).map(Frame::Oci).map_err(FrameError::Oci),
            AUTHORIZE => Ok(Frame::Authorize(rest)),
            tag => Err(FrameError::UnknownTag(tag)),
        }
    }
}

/// A creation channel request that could not be read.
#[derive(Debug)]
pub enum FrameError {
    /// No bytes at all, so not even a tag.
    Empty,
    /// A tag that is neither [`Frame::Oci`] nor [`Frame::Authorize`].
    UnknownTag(u8),
    /// The registry request did not parse.
    Oci(serde_json::Error),
}

impl fmt::Display for FrameError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FrameError::Empty => {
                f.write_str("creation channel request frame is empty")
            }
            FrameError::UnknownTag(tag) => {
                write!(f, "unknown creation channel request tag {tag}")
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
