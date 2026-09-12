//! The provider asking for a write's content.

use std::error::Error;
use std::fmt;

use crate::decode::Decode;
use crate::encode::{Encode, Writer};

/// Send the content for a write.
///
/// Opened by the provider, on its own channel, in answer to a
/// [`write_path::request::Request`](crate::shared::containers::write_path::request::Request)
/// the client opened on one of its.
///
/// # Why it names the write
///
/// Because a client may have several in flight, and this channel is
/// not the one the write arrived on — only a responder can finish a
/// channel, so the provider had to open one of its own to ask.
///
/// Neither side's header can name the other's channels: they are
/// numbered per SENDER, so a number in one namespace means nothing in
/// the other. The
/// [`write_id`](crate::shared::containers::write_path::request::Request::write_id)
/// belongs to the write instead of to either channel, which is what
/// lets both ends read it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Request {
    /// The write being asked about, quoted from the
    /// [`write_path::request::Request`](crate::shared::containers::write_path::request::Request)
    /// that opened it.
    pub write_id: u32,
}

/// The bytes a write id occupies.
const WRITE_ID_LEN: usize = 4;

/// Four big-endian bytes. No serialization, because a fixed-width
/// integer does not need one.
impl Encode for Request {
    /// [`Infallible`](std::convert::Infallible): four known bytes.
    type Error = std::convert::Infallible;

    fn encode(&self, out: &mut Writer<'_>) -> Result<(), Self::Error> {
        out.extend_from_slice(&self.write_id.to_be_bytes());
        Ok(())
    }
}

impl Decode<'_> for Request {
    /// One way to fail: the wrong number of bytes.
    type Error = RequestError;

    fn decode(bytes: &[u8]) -> Result<Self, Self::Error> {
        <[u8; WRITE_ID_LEN]>::try_from(bytes)
            .map(|bytes| Request {
                write_id: u32::from_be_bytes(bytes),
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
