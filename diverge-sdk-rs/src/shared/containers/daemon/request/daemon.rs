//! Which daemon connection a channel belongs to.

use std::error::Error;
use std::fmt;

use crate::wire::decode::Decode;
use crate::wire::encode::{Encode, Writer};

/// One daemon connection, by the id the proxy minted when the program
/// dialed `/daemon`: what both halves of the pair carry, and the only
/// thing that ties them together. See
/// [`daemon`](crate::shared::containers::daemon) for the pair.
///
/// Four bytes, the id big-endian, and nothing else: the id is unique
/// among the connections the proxy has announced and the caller has
/// not finished, and the proxy does not reuse one while either half
/// is open.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Daemon {
    /// The connection, by the id the proxy minted.
    pub connection_id: u32,
}

/// The bytes an id occupies.
const CONNECTION_ID_LEN: usize = 4;

impl Encode for Daemon {
    /// [`Infallible`](std::convert::Infallible): four bytes copied.
    type Error = std::convert::Infallible;

    fn encode(&self, out: &mut Writer<'_>) -> Result<(), Self::Error> {
        out.extend_from_slice(&self.connection_id.to_be_bytes());
        Ok(())
    }
}

impl Decode<'_> for Daemon {
    /// The one way four bytes fail.
    type Error = DaemonError;

    fn decode(bytes: &[u8]) -> Result<Self, Self::Error> {
        <[u8; CONNECTION_ID_LEN]>::try_from(bytes)
            .map(|bytes| Daemon {
                connection_id: u32::from_be_bytes(bytes),
            })
            .map_err(|_| DaemonError::Length(bytes.len()))
    }
}

/// A daemon connection ask that could not be read.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DaemonError {
    /// Not exactly four bytes.
    Length(usize),
}

impl fmt::Display for DaemonError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DaemonError::Length(length) => write!(f, "daemon connection ask is {length} bytes, not {CONNECTION_ID_LEN}"),
        }
    }
}

impl Error for DaemonError {}
