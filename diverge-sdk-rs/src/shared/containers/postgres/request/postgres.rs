//! Naming a connection.

use std::error::Error;
use std::fmt;

use crate::wire::decode::Decode;
use crate::wire::encode::{Encode, Writer};

/// Which connection a channel is one half of.
///
/// The provider opens its half carrying this with an id it minted,
/// and the caller opens the other half quoting the same id. That is
/// the whole of the correlation: a caller with several connections in
/// flight learns which one it is being asked to feed.
///
/// # It is the provider's to choose and the provider's to keep unique
///
/// Unique among the connections this provider has open in this scope
/// — reusing one that is still live makes two connections
/// indistinguishable, and the provider is the only party that could
/// have prevented it. A number that counts up is the obvious way and
/// nothing requires it. Reusing one after both channels have finished
/// is fine; nothing here remembers.
///
/// # Why not the channel it arrived on
///
/// Because channels are numbered per SENDER. The provider opens its
/// half on a channel of its own, the caller opens the other on a
/// channel of its own, and neither side's header can name the other's
/// — so a payload quoting a channel number would be quoting one out of
/// a namespace its reader does not share. An id invented for the
/// connection belongs to neither channel, which is what makes it
/// readable on both sides — the same argument a
/// [`write_id`](crate::shared::containers::write_path::request::Request::write_id)
/// makes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Postgres {
    /// What this connection is called, chosen by the provider.
    pub connection_id: u32,
}

/// The bytes a connection id occupies.
const CONNECTION_ID_LEN: usize = 4;

/// Four big-endian bytes. No serialization, because a fixed-width
/// integer does not need one.
impl Encode for Postgres {
    /// [`Infallible`](std::convert::Infallible): four known bytes.
    type Error = std::convert::Infallible;

    fn encode(&self, out: &mut Writer<'_>) -> Result<(), Self::Error> {
        out.extend_from_slice(&self.connection_id.to_be_bytes());
        Ok(())
    }
}

impl Decode<'_> for Postgres {
    /// One way to fail: the wrong number of bytes.
    type Error = PostgresError;

    fn decode(bytes: &[u8]) -> Result<Self, Self::Error> {
        <[u8; CONNECTION_ID_LEN]>::try_from(bytes)
            .map(|bytes| Postgres {
                connection_id: u32::from_be_bytes(bytes),
            })
            .map_err(|_| PostgresError::Length(bytes.len()))
    }
}

/// A Postgres channel request that could not be read.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PostgresError {
    /// A payload that was not four bytes, carrying however many there
    /// were.
    Length(usize),
}

impl fmt::Display for PostgresError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PostgresError::Length(len) => {
                write!(f, "postgres connection request is {len} bytes, not 4")
            }
        }
    }
}

impl Error for PostgresError {}
