//! What a client's request frame carries for a connection.

use std::error::Error;
use std::fmt;
use std::str::{self, Utf8Error};

use crate::decode::Decode;
use crate::encode::{Encode, Writer};

/// Ask to join a container somebody else created.
///
/// # What happens to the authorization
///
/// Nothing, here. The provider relays it to whoever holds the
/// container's run scope, as an
/// [`Authorize`](crate::endpoints::laboratories::run::server::channel_request::Frame::Authorize),
/// and the answer to that is whether this scope opens.
///
/// Which is why the credential is opaque. A provider that had to
/// understand it would have to know what makes one connector
/// acceptable and another not, and it does not — the runner does, and
/// the runner is who reads it.
///
/// # The layout
///
/// Neither JSON nor postcard. The frame is:
///
/// ```text
/// [tag: u8][id length: u16 big-endian][id: utf-8][authorization…]
/// ```
///
/// One length, because there are two variable-length fields and the
/// frame's own end delimits the second. The authorization runs to
/// wherever the payload stops.
///
/// Both fields are strings, so a format would work now — the trap that
/// once ruled postcard out was serde writing `&[u8]` as a sequence and
/// reading it back as a byte string, and there are no bytes here any
/// more. The layout stays because nothing about two strings and a
/// length is a shape a format would improve, and because both fields
/// are borrowed straight out of the frame: JSON would have to unescape
/// them into owned copies to hand either one back.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Frame<'a> {
    /// The container to join.
    ///
    /// A [`Frame::Id`](crate::endpoints::laboratories::run::server::response::Frame::Id)
    /// from a run. It means nothing to a connector that was not
    /// given it, and nothing outside the provider that minted it.
    pub id: &'a str,
    /// Whatever the runner needs in order to say yes.
    ///
    /// Opaque, and relayed verbatim. A shared secret, a signed token,
    /// a name — this layer does not know and does not look, so nothing
    /// here constrains what a runner chooses to require.
    ///
    /// Text, for the same reason an
    /// [`Auth`](crate::auth::Auth) credential is: what this carries in
    /// practice already is a string, and bytes made a caller pick an
    /// encoding for something that never needed one.
    ///
    /// May be empty, which is a connector offering nothing. Whether
    /// that is ever enough is the runner's to decide.
    pub authorization: &'a str,
}

/// This frame's tag among the scope-opening requests.
///
/// One byte at the front of the payload, which is what tells a reader
/// which request it holds. The frame layer does not discriminate them
/// — [`ClientFrame::Request`](crate::frame::client::ClientFrame::Request)
/// is one type carrying bytes — so the distinction has to be in the
/// bytes, and each request owns the value that names it.
///
/// See the table in [`endpoints`](crate::endpoints) for the whole
/// allocation. The values are chosen across modules that do not know
/// about each other, so the table is the only place they can be seen
/// at once.
const TAG: u8 = 5;

/// The bytes an id's length occupies.
const LENGTH_LEN: usize = 2;

impl Encode for Frame<'_> {
    /// One way to fail, and it is not a serialization.
    type Error = FrameEncodeError;

    fn encode(&self, out: &mut Writer<'_>) -> Result<(), Self::Error> {
        let length = u16::try_from(self.id.len())
            .map_err(|_| FrameEncodeError::IdTooLong(self.id.len()))?;
        out.extend_from_slice(&[TAG]);
        out.extend_from_slice(&length.to_be_bytes());
        out.extend_from_slice(self.id.as_bytes());
        out.extend_from_slice(self.authorization.as_bytes());
        Ok(())
    }
}

impl<'a> Decode<'a> for Frame<'a> {
    /// Four ways to fail, and none of them is a parse.
    type Error = FrameDecodeError;

    fn decode(bytes: &'a [u8]) -> Result<Self, Self::Error> {
        let (tag, rest) =
            bytes.split_first().ok_or(FrameDecodeError::Empty)?;
        if *tag != TAG {
            return Err(FrameDecodeError::UnexpectedTag(*tag));
        }
        let (length, rest) = rest
            .split_at_checked(LENGTH_LEN)
            .ok_or(FrameDecodeError::Truncated)?;
        let length = usize::from(u16::from_be_bytes([length[0], length[1]]));
        let (id, authorization) = rest
            .split_at_checked(length)
            .ok_or(FrameDecodeError::Truncated)?;
        let id = str::from_utf8(id).map_err(FrameDecodeError::Id)?;
        let authorization = str::from_utf8(authorization)
            .map_err(FrameDecodeError::Authorization)?;
        Ok(Frame { id, authorization })
    }
}

/// A connection request that could not be written.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FrameEncodeError {
    /// An id longer than a `u16` can measure, carrying its length.
    ///
    /// Two bytes of length is 65535 characters of id, which is not a
    /// limit anything sane meets. It is an error rather than a wider
    /// field because the alternative is spending two more bytes on
    /// every connection to describe ids nobody mints.
    IdTooLong(usize),
}

impl fmt::Display for FrameEncodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FrameEncodeError::IdTooLong(length) => {
                write!(f, "container id is {length} bytes, over 65535")
            }
        }
    }
}

impl Error for FrameEncodeError {}

/// A connection request that could not be read.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrameDecodeError {
    /// No bytes at all, so not even a tag.
    Empty,
    /// A tag naming some other request.
    UnexpectedTag(u8),
    /// The payload ended inside the length or inside the id.
    Truncated,
    /// The id was not UTF-8.
    Id(Utf8Error),
    /// The authorization was not UTF-8.
    Authorization(Utf8Error),
}

impl fmt::Display for FrameDecodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FrameDecodeError::Empty => {
                f.write_str("connection request frame is empty")
            }
            FrameDecodeError::UnexpectedTag(tag) => {
                write!(f, "expected connection request tag {TAG}, found {tag}")
            }
            FrameDecodeError::Truncated => {
                f.write_str("connection request ended inside its id")
            }
            FrameDecodeError::Id(error) => {
                write!(f, "container id is not utf-8: {error}")
            }
            FrameDecodeError::Authorization(error) => {
                write!(f, "authorization is not utf-8: {error}")
            }
        }
    }
}

impl Error for FrameDecodeError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            FrameDecodeError::Id(error)
            | FrameDecodeError::Authorization(error) => Some(error),
            FrameDecodeError::Empty
            | FrameDecodeError::UnexpectedTag(_)
            | FrameDecodeError::Truncated => None,
        }
    }
}
