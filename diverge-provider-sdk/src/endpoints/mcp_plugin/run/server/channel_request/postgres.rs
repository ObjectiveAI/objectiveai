//! The provider asking for a database connection's output.

use std::error::Error;
use std::fmt;

use crate::decode::Decode;
use crate::encode::{Encode, Writer};

/// Connect to the database, and stream back what it says.
///
/// The first half of a Postgres connection. A provider opens a channel
/// with this because something inside the container dialled the
/// conduit it was given, and the database lives with the caller.
///
/// What comes back on this channel is everything the DATABASE says.
/// What the plugin writes travels the other way, on a channel the
/// CALLER opens — see
/// [`client::channel_request::Postgres`](crate::endpoints::mcp_plugin::run::client::channel_request::Postgres).
///
/// # Why a connection is two channels
///
/// Because only a responder can end a channel, and a connection has to
/// be endable from both sides. A plugin that dies has to be sayable to
/// the caller, or the caller's backend stays open with nothing left to
/// serve; a database that drops has to be sayable to the provider, or
/// the plugin waits on a reply that is not coming.
///
/// One duplex channel could express neither. Two channels express
/// both, with nothing added: each side finishes the one it is
/// answering on, and that finish IS the close.
///
/// It is the same inversion a
/// [`write`](crate::shared::container::write_path) makes, for the same
/// reason and at the same price — one round trip before the first
/// byte.
///
/// # It is opened, not offered
///
/// Twice over. A plugin that named no
/// [`postgres_port`](crate::endpoints::mcp_plugin::run::client::request::Frame::postgres_port)
/// never has one of these at all, because a provider has nothing to
/// dial. And one that named a port but has no queries to send does not
/// either, because nothing inside it opened a connection.
///
/// So an opted-out plugin costs nothing rather than costing an idle
/// tunnel, and a quiet one costs nothing either.
///
/// # Several at once is the ordinary case
///
/// A plugin holds a connection pool, so a provider opens a pair per
/// connection and they run in parallel. Nothing is shared between
/// them: each pair has its own
/// [`connection_id`](Self::connection_id), its own two channels, and
/// its own ordering. Frames belonging to different connections
/// interleave freely on the socket, which is what keeps one large
/// result set from blocking its siblings.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Postgres {
    /// What this connection is called, chosen by the provider.
    ///
    /// The caller quotes it back in the
    /// [`client::channel_request::Postgres`](crate::endpoints::mcp_plugin::run::client::channel_request::Postgres)
    /// that asks for the plugin's writes, and that is the whole of the
    /// correlation: a caller with several connections in flight learns
    /// which one it is being asked to feed.
    ///
    /// # It is the provider's to choose and the provider's to keep
    /// unique
    ///
    /// Unique among the connections this provider has open in this
    /// scope — reusing one that is still live makes two connections
    /// indistinguishable, and the provider is the only party that
    /// could have prevented it. A number that counts up is the obvious
    /// way and nothing requires it.
    ///
    /// Reusing one after both channels have finished is fine. Nothing
    /// here remembers.
    ///
    /// # Why not the channel it arrived on
    ///
    /// Because channels are numbered per SENDER. The provider opens
    /// this on a channel of its own, the caller asks for the writes on
    /// a channel of its own, and neither side's header can name the
    /// other's — so a payload quoting a channel number would be
    /// quoting one out of a namespace its reader does not share.
    ///
    /// An id invented for the connection belongs to neither channel,
    /// which is what makes it readable on both sides. The same
    /// argument a
    /// [`write_id`](crate::shared::container::write_path::request::Request::write_id)
    /// makes.
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
