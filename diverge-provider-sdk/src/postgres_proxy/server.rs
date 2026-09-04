//! Frames the server sends.

use std::convert::Infallible;

use super::FrameError;
use crate::encode::{Encode, Writer};

/// A frame sent by the server — the provider, on the connection it
/// opened into the container — about a connection the container
/// announced.
///
/// ```text
/// [type: u8][connection: u32][payload…]
/// ```
///
/// Two kinds: the database's bytes, and the database's end. The
/// server opens nothing — the container is the only minter on this
/// wire — so there is no `open` in this direction.
#[derive(Debug, Clone, PartialEq)]
pub enum Frame<'a> {
    /// Type `0`. Bytes the database said on the connection: pgwire,
    /// verbatim, never parsed, in the order said. Any number of
    /// these; a pgwire message may span several.
    ///
    /// On the wire from the caller these were the responses on the
    /// server's channel request —
    /// [`client::channel_response::postgres`](crate::endpoints::agentic_loop::run::client::channel_response::postgres).
    Data {
        /// The connection said on.
        connection: u32,
        /// What was said.
        payload: &'a [u8],
    },
    /// Type `1`. The database end of the connection closed — or the
    /// caller declined to dial it at all, which looks the same from
    /// here. The proxy acts on it by shutting the agent's socket: the
    /// driver sees a server that hung up on it, which is the truth.
    ///
    /// On the wire from the caller this was the finish on the
    /// server's channel request — with no response before it, the
    /// decline.
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
            Frame::Data {
                connection,
                payload,
            } => {
                out.extend_from_slice(&[0]);
                out.extend_from_slice(&connection.to_be_bytes());
                out.extend_from_slice(payload);
            }
            Frame::Close { connection } => {
                out.extend_from_slice(&[1]);
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
            0 => Ok(Frame::Data {
                connection,
                payload,
            }),
            1 => Ok(Frame::Close { connection }),
            other => Err(FrameError::UnknownType(other)),
        }
    }
}
