//! Frames the container sends.

use std::convert::Infallible;

use super::FrameError;
use crate::encode::{Encode, Writer};

/// A frame sent by the container — the proxy, about a connection the
/// agent's driver opened on it.
///
/// ```text
/// [type: u8][connection: u32][payload…]
/// ```
///
/// Three kinds, which is why there is a type byte where the MCP
/// proxy's container frame has none: a connection is announced,
/// written on, and closed, and the same header names which.
#[derive(Debug, Clone, PartialEq)]
pub enum Frame<'a> {
    /// Type `0`. A connection the agent just opened: the proxy
    /// accepted a socket and minted the number. No payload — the
    /// connection's first bytes follow as [`Data`](Self::Data), and
    /// pgwire is client-first, so they follow at once.
    ///
    /// On the wire to the caller this is the server opening its
    /// [`Postgres`](crate::endpoints::agentic_loop::run::server::channel_request::Postgres)
    /// channel request, quoting this very number as the
    /// `connection_id`.
    Open {
        /// The connection, unique among the container's live ones.
        connection: u32,
    },
    /// Type `1`. Bytes the agent wrote on the connection: pgwire,
    /// verbatim, never parsed, in the order written. There may be any
    /// number of these, and a pgwire message may span several.
    ///
    /// On the wire to the caller these are the server's responses on
    /// the channel the caller opened for the connection's writes —
    /// [`server::channel_response::postgres`](crate::endpoints::agentic_loop::run::server::channel_response::postgres).
    Data {
        /// The connection written on.
        connection: u32,
        /// What was written.
        payload: &'a [u8],
    },
    /// Type `2`. The agent's socket ended: no further byte will ever
    /// be written on this connection, because the thing writing them
    /// is gone. Whether it ended cleanly (pgwire's `Terminate` came
    /// first) or crashed is not said, and does not matter — either
    /// way the caller's backend is released.
    ///
    /// On the wire to the caller this is the server's finish on the
    /// channel the caller opened.
    Close {
        /// The connection that ended.
        connection: u32,
    },
}

/// A frame writes its own header, and a payload never does — the same
/// split the main protocol's frames make, for the same reason.
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
