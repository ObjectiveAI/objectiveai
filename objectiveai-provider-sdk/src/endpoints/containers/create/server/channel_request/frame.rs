//! What a server's channel request frame carries for a creation.

use std::error::Error;
use std::fmt;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

use super::Authorize;
use crate::decode::Decode;
use crate::encode::{Encode, Writer};
use crate::shared::http::request::Request;

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
    /// [`ImageType::Client`](crate::endpoints::containers::create::client::request::ImageType::Client)
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
    /// Ask the caller whether a connector may attach to the container.
    ///
    /// Opened when one arrives. A yes is what
    /// [`Connections`](crate::endpoints::containers::create::server::response::Frame::Connections)
    /// then reflects; a no is a connector that never joins.
    ///
    /// This layer guarantees two things and no more — that the bytes
    /// arrive as they were sent, and that the question is answered
    /// before the connection it is about is allowed to open.
    Authorize(Authorize<'a>),
}

/// Tag for [`Frame::Oci`].
const OCI: u8 = 0;

/// Tag for [`Frame::Authorize`].
const AUTHORIZE: u8 = 1;

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
                out.extend_from_slice(authorize.authorization);
                Ok(())
            }
        }
    }
}

impl<'a> Decode<'a> for Frame<'a> {
    /// Five ways to fail, and only one of them is a parse.
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
                Ok(Frame::Authorize(Authorize {
                    address,
                    authorization,
                }))
            }
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
    /// An authorization that ended inside its address.
    Truncated,
    /// An address version byte that is neither `4` nor `6`.
    UnknownAddress(u8),
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
            FrameError::Truncated => {
                f.write_str("authorization ended inside its address")
            }
            FrameError::UnknownAddress(version) => {
                write!(f, "address version is neither {V4} nor {V6}: {version}")
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
            FrameError::Empty
            | FrameError::UnknownTag(_)
            | FrameError::Truncated
            | FrameError::UnknownAddress(_) => None,
        }
    }
}
