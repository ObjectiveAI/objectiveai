//! Frames the container sends on `/postgres`.

use std::convert::Infallible;

use super::FrameError;
use crate::encode::{Encode, Writer};

/// A frame sent by the container — the proxy, about a connection
/// its driver opened on the loopback listener.
///
/// ```text
/// [type: u8][connection: u32][payload…]
/// ```
///
/// Three kinds: a connection is announced, written on, and closed,
/// and the same header names which.
#[derive(Debug, Clone, PartialEq)]
pub enum Frame<'a> {
    /// Type `0`. A connection the driver just opened: the proxy
    /// accepted a socket and minted the number. No payload — the
    /// connection's first bytes follow as [`Data`](Self::Data), and
    /// pgwire is client-first, so they follow at once.
    Open {
        /// The connection, unique among the container's live ones.
        connection: u32,
    },
    /// Type `1`. Bytes the driver wrote on the connection: pgwire,
    /// verbatim, never parsed, in the order written. Any number of
    /// these; a pgwire message may span several.
    Data {
        /// The connection written on.
        connection: u32,
        /// What was written.
        payload: &'a [u8],
    },
    /// Type `2`. The driver's socket ended: no further byte will ever
    /// be written on this connection, because the thing writing them
    /// is gone. Whether it ended cleanly (pgwire's `Terminate` came
    /// first) or crashed is not said, and does not matter — either
    /// way the caller's backend is released.
    Close {
        /// The connection that ended.
        connection: u32,
    },
}

/// A frame writes its own header, and a payload never does.
impl Encode for Frame<'_> {
    /// [`Infallible`]: a header is fixed bytes and the payload is bytes
    /// already.
    type Error = Infallible;

    fn encode(&self, out: &mut Writer<'_>) -> Result<(), Infallible> {
        match self {
            Frame::Open { connection } => {
                out.extend_from_slice(&[0]);
                out.extend_from_slice(&connection.to_be_bytes());
            }
            Frame::Data {
                connection,
                payload,
            } => {
                out.extend_from_slice(&[1]);
                out.extend_from_slice(&connection.to_be_bytes());
                out.extend_from_slice(payload);
            }
            Frame::Close { connection } => {
                out.extend_from_slice(&[2]);
                out.extend_from_slice(&connection.to_be_bytes());
            }
        }
        Ok(())
    }
}

impl<'a> Frame<'a> {
    /// Decode one frame from a WebSocket message's binary payload.
    /// The payload borrows from `bytes`.
    pub fn decode(bytes: &'a [u8]) -> Result<Self, FrameError> {
        let (r#type, connection, payload) = super::split_header(bytes)?;
        match r#type {
            0 => Ok(Frame::Open { connection }),
            1 => Ok(Frame::Data {
                connection,
                payload,
            }),
            2 => Ok(Frame::Close { connection }),
            other => Err(FrameError::UnknownType(other)),
        }
    }
}
