//! One server frame that answers a client frame, or the news that none
//! will.

use super::{FrameError, Served};
use crate::wire::decode::Decode as _;
use crate::wire::encode::{Encode, Writer};
use crate::wire::frame::server::ServerFrame;
use crate::shared::error::Error;

/// One server frame of the daemon connection, or the error.
///
/// A message leads with one byte saying which — `0` for
/// [`Served`](Self::Served), `1` for [`Error`](Self::Error) — and the
/// rest is that variant's own bytes: the server frame as the wire lays
/// it out, or the error's JSON.
///
/// # An error is not a frame
///
/// [`Error`](Self::Error) is the caller saying the client frame could
/// not be served at all — the container's session is gone, the frame
/// names a scope it does not have — which the daemon's own frames have
/// no way to say, since on a connection a frame nobody can serve is
/// simply not answered. What arrived before it stands; nothing follows
/// it, because the finish comes after; and the proxy turns it into the
/// wire's word for "not served" toward the program, as
/// [`daemon`](crate::shared::containers::daemon) states.
#[derive(Debug, Clone, PartialEq)]
pub enum Frame<'a> {
    /// One server frame, borrowed from the message it arrived in. Tag
    /// `0`.
    Served(Served<'a>),
    /// The client frame could not be served. Tag `1`.
    ///
    /// See [`shared::error::Error`](crate::shared::error::Error) for
    /// why it says so little.
    Error(Error),
}

/// Tag for [`Frame::Served`].
const SERVED: u8 = 0;

/// Tag for [`Frame::Error`].
const ERROR: u8 = 1;

impl Encode for Frame<'_> {
    /// The ordinary JSON failure, from the only variant that has one.
    /// A frame is bytes copied.
    type Error = serde_json::Error;

    // Spelled out rather than `Self::Error`: this enum has a variant
    // called `Error`, so the associated type is ambiguous by that name.
    fn encode(&self, out: &mut Writer<'_>) -> Result<(), serde_json::Error> {
        match self {
            Frame::Served(served) => {
                out.extend_from_slice(&[SERVED]);
                ServerFrame::from(*served).encode(out).map_err(|error| match error {})
            }
            Frame::Error(error) => {
                out.extend_from_slice(&[ERROR]);
                error.encode(out)
            }
        }
    }
}

impl<'a> Frame<'a> {
    /// Decode one message. A frame borrows from `bytes`.
    pub fn decode(bytes: &'a [u8]) -> Result<Self, FrameError> {
        let (tag, rest) = bytes.split_first().ok_or(FrameError::Empty)?;
        match *tag {
            SERVED => {
                let frame = ServerFrame::decode(rest).map_err(FrameError::Frame)?;
                Served::try_from(frame).map(Frame::Served).map_err(|_| FrameError::Auth)
            }
            ERROR => Error::decode(rest).map(Frame::Error).map_err(FrameError::Error),
            tag => Err(FrameError::UnknownTag(tag)),
        }
    }
}
