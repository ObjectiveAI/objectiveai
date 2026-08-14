//! The provider asking for a write's content.

use std::error::Error;
use std::fmt;

use crate::decode::Decode;
use crate::encode::{Encode, Writer};

/// Send the content for a write.
///
/// Opened by the provider, on its own channel, in answer to a
/// [`write_path::request::Request`](crate::shared::container::write_path::request::Request)
/// the client opened on one of its.
///
/// # Why it names a channel
///
/// Because a client may have several writes in flight, and the two
/// channels live in different numbering spaces — the provider mints
/// this one, the client minted the other, and neither can guess the
/// other's. So the ask carries the client's channel, and that is the
/// whole of the correlation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Request {
    /// The channel the client opened its write on, in the CLIENT's
    /// numbering.
    pub channel: u32,
}

/// The bytes a channel number occupies.
const CHANNEL_LEN: usize = 4;

/// Four big-endian bytes, the same way `scope` and `channel` are
/// written in every header. No serialization, because a fixed-width
/// integer does not need one.
impl Encode for Request {
    /// [`Infallible`](std::convert::Infallible): four known bytes.
    type Error = std::convert::Infallible;

    fn encode(&self, out: &mut Writer<'_>) -> Result<(), Self::Error> {
        out.extend_from_slice(&self.channel.to_be_bytes());
        Ok(())
    }
}

impl Decode<'_> for Request {
    /// One way to fail: the wrong number of bytes.
    type Error = RequestError;

    fn decode(bytes: &[u8]) -> Result<Self, Self::Error> {
        <[u8; CHANNEL_LEN]>::try_from(bytes)
            .map(|bytes| Request {
                channel: u32::from_be_bytes(bytes),
            })
            .map_err(|_| RequestError::Length(bytes.len()))
    }
}

/// A content request that could not be read.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RequestError {
    /// A payload that was not four bytes, carrying however many there
    /// were.
    Length(usize),
}

impl fmt::Display for RequestError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RequestError::Length(len) => {
                write!(f, "write content request is {len} bytes, not 4")
            }
        }
    }
}

impl Error for RequestError {}
