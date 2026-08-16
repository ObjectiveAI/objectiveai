//! What a server's channel request frame carries for a laboratory run.

use std::error::Error;
use std::fmt;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
use std::str::{self, Utf8Error};

use super::Authorize;
use crate::decode::Decode;
use crate::encode::{Encode, Writer};
use crate::shared::container::write_bytes;
use crate::shared::http::request::Request;

/// What a provider asks a caller for while a laboratory runs.
///
/// A payload leads with one byte saying which — `0` for
/// [`Oci`](Self::Oci), `1` for [`Authorize`](Self::Authorize), `2` for
/// [`Write`](Self::Write) — and the rest is that variant's own bytes. The frame's own `type` could
/// have carried this and deliberately does not: a frame already
/// carries one payload's worth of protocol, and splitting the
/// discrimination across the envelope and the payload would mean two
/// vocabularies to version and two places to keep in step.
#[derive(Debug, Clone, PartialEq)]
pub enum Frame<'a> {
    /// One request against the caller's registry.
    ///
    /// Opened only for an
    /// [`Image::Client`](crate::shared::container::request::Image::Client)
    /// laboratory, and opened by the container RUNTIME's appetite rather
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
    /// endpoint serves every run happening at once and a request
    /// routes itself without a provider keeping state between them.
    Oci(Request<'a>),
    /// Ask the caller whether a connector may attach to the container.
    ///
    /// Opened when one arrives. A yes lets it attach and names it —
    /// see
    /// [`Authorized`](crate::endpoints::laboratories::run::client::channel_response::authorize::Frame::Authorized)
    /// — and that name comes back as a
    /// [`Disconnected`](crate::endpoints::laboratories::run::server::response::Frame::Disconnected)
    /// when it leaves. A no is a connector that never joins.
    ///
    /// This layer guarantees two things and no more — that the bytes
    /// arrive as they were sent, and that the question is answered
    /// before the connection it is about is allowed to open.
    Authorize(Authorize<'a>),
    /// Send the content for a write. Tag `2`.
    ///
    /// Opened in answer to a
    /// [`Write`](crate::endpoints::laboratories::run::client::channel_request::Frame::Write)
    /// the caller started. A write cannot carry its own content —
    /// only a responder can finish a channel — so the bytes travel as
    /// responses on this one.
    Write(write_bytes::request::Request),
}

/// Tag for [`Frame::Oci`].
const OCI: u8 = 0;

/// Tag for [`Frame::Authorize`].
const AUTHORIZE: u8 = 1;

/// Tag for [`Frame::Write`].
const WRITE: u8 = 2;

/// Marks an [`IpAddr::V4`].
const V4: u8 = 4;

/// Marks an [`IpAddr::V6`].
const V6: u8 = 6;

impl Encode for Frame<'_> {
    /// Only [`Oci`](Frame::Oci) can fail, and only the way any JSON
    /// serialization can.
    ///
    /// An authorization cannot. Its address is fixed-width and its
    /// payload is bytes, which is the point of laying it out by hand:
    /// there is no length that could overflow a prefix and no shape a
    /// format could reject.
    type Error = serde_json::Error;

    fn encode(&self, out: &mut Writer<'_>) -> Result<(), Self::Error> {
        match self {
            Frame::Oci(request) => {
                out.extend_from_slice(&[OCI]);
                request.encode(out)
            }
            Frame::Authorize(authorize) => {
                out.extend_from_slice(&[AUTHORIZE]);
                match authorize.address {
                    IpAddr::V4(address) => {
                        out.extend_from_slice(&[V4]);
                        out.extend_from_slice(&address.octets());
                    }
                    IpAddr::V6(address) => {
                        out.extend_from_slice(&[V6]);
                        out.extend_from_slice(&address.octets());
                    }
                }
                out.extend_from_slice(authorize.authorization.as_bytes());
                Ok(())
            }
            Frame::Write(request) => {
                out.extend_from_slice(&[WRITE]);
                // Its error is `Infallible`, and an empty match on one
                // is how you say so: there is no value to handle.
                request.encode(out).map_err(|error| match error {})
            }
        }
    }
}

impl<'a> Decode<'a> for Frame<'a> {
    /// Seven ways to fail, and only one of them is a parse.
    type Error = FrameError;

    fn decode(bytes: &'a [u8]) -> Result<Self, Self::Error> {
        let (tag, rest) = bytes.split_first().ok_or(FrameError::Empty)?;
        match *tag {
            OCI => {
                Request::decode(rest).map(Frame::Oci).map_err(FrameError::Oci)
            }
            AUTHORIZE => {
                let (version, rest) =
                    rest.split_first().ok_or(FrameError::Truncated)?;
                let (address, authorization) = match *version {
                    V4 => {
                        let (octets, rest) = rest
                            .split_at_checked(4)
                            .ok_or(FrameError::Truncated)?;
                        let octets = <[u8; 4]>::try_from(octets)
                            .map_err(|_| FrameError::Truncated)?;
                        (IpAddr::V4(Ipv4Addr::from(octets)), rest)
                    }
                    V6 => {
                        let (octets, rest) = rest
                            .split_at_checked(16)
                            .ok_or(FrameError::Truncated)?;
                        let octets = <[u8; 16]>::try_from(octets)
                            .map_err(|_| FrameError::Truncated)?;
                        (IpAddr::V6(Ipv6Addr::from(octets)), rest)
                    }
                    version => {
                        return Err(FrameError::UnknownAddress(version));
                    }
                };
                let authorization = str::from_utf8(authorization)
                    .map_err(FrameError::Authorization)?;
                Ok(Frame::Authorize(Authorize {
                    address,
                    authorization,
                }))
            }
            WRITE => write_bytes::request::Request::decode(rest)
                .map(Frame::Write)
                .map_err(FrameError::Write),
            tag => Err(FrameError::UnknownTag(tag)),
        }
    }
}

/// A laboratory run channel request that could not be read.
#[derive(Debug)]
pub enum FrameError {
    /// No bytes at all, so not even a tag.
    Empty,
    /// A tag that is none of this frame's three.
    UnknownTag(u8),
    /// An authorization that ended inside its address.
    Truncated,
    /// An address version byte that is neither `4` nor `6`.
    UnknownAddress(u8),
    /// The authorization was not UTF-8.
    Authorization(Utf8Error),
    /// The registry request did not parse.
    Oci(serde_json::Error),
    /// The write content request did not decode.
    Write(write_bytes::request::RequestError),
}

impl fmt::Display for FrameError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FrameError::Empty => {
                f.write_str("laboratory run channel request frame is empty")
            }
            FrameError::UnknownTag(tag) => {
                write!(f, "unknown laboratory run channel request tag {tag}")
            }
            FrameError::Truncated => {
                f.write_str("authorization ended inside its address")
            }
            FrameError::UnknownAddress(version) => {
                write!(f, "address version is neither {V4} nor {V6}: {version}")
            }
            FrameError::Authorization(error) => {
                write!(f, "authorization is not utf-8: {error}")
            }
            FrameError::Oci(error) => {
                write!(f, "registry request did not parse: {error}")
            }
            FrameError::Write(error) => {
                write!(f, "write content request did not decode: {error}")
            }
        }
    }
}

impl Error for FrameError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            FrameError::Oci(error) => Some(error),
            FrameError::Write(error) => Some(error),
            FrameError::Authorization(error) => Some(error),
            FrameError::Empty
            | FrameError::UnknownTag(_)
            | FrameError::Truncated
            | FrameError::UnknownAddress(_) => None,
        }
    }
}
