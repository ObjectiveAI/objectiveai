//! The provider asking for a database connection's output.

use std::error::Error;
use std::fmt;

use crate::decode::Decode;
use crate::encode::{Encode, Writer};

/// Connect to the database, and stream back what it says.
///
/// The first half of a Postgres connection. A provider opens a channel
/// with this because the agent's container opened a database
/// connection — on its own loopback, port `14980`, the container's
/// Postgres proxy, which announced it to the server over the
/// [`postgres_proxy`](crate::postgres_proxy) wire — and the database
/// lives with the caller.
///
/// What comes back on this channel is everything the DATABASE says.
/// What the container writes travels the other way, on a channel the
/// CALLER opens — see
/// [`client::channel_request::Postgres`](crate::endpoints::agentic_loop::run::client::channel_request::Postgres).
///
/// The same exchange the plugin endpoint carries, as
/// [`mcp_plugin`'s](crate::endpoints::mcp_plugin::run::server::channel_request::Postgres):
/// same four bytes, same pair, same finishes. It is written out
/// here rather than shared because a connection id is this
/// endpoint's own and means nothing outside it.
///
/// # Why a connection is two channels
///
/// Because only a responder can end a channel, and a connection has to
/// be endable from both sides. An agent that dies has to be sayable
/// to the caller, or the caller's backend stays open with nothing left
/// to serve; a database that drops has to be sayable to the provider,
/// or the agent waits on a reply that is not coming.
///
/// One duplex channel could express neither. Two channels express
/// both, with nothing added: each side finishes the one it is
/// answering on, and that finish IS the close.
///
/// # It is opened, not offered
///
/// Nothing in the request declares it. A container that never dials
/// `14980` never has one of these at all, and an upstream that keeps
/// its state elsewhere costs nothing rather than costing an idle
/// tunnel. The one that does — an agent whose memory is rows, which
/// is what put this on the loop — opens one per connection its pool
/// makes.
///
/// # Several at once is the ordinary case
///
/// A database client holds a pool, so a provider opens a pair per
/// connection and they run in parallel. Nothing is shared between
/// them: each pair has its own
/// [`connection_id`](Self::connection_id), its own two channels, and
/// its own ordering. Frames belonging to different connections
/// interleave freely on the socket, which is what keeps one large
/// result set from blocking its siblings.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Postgres {
    /// What this connection is called, chosen by the provider — in
    /// practice the number the container's proxy minted when it
    /// announced the connection, carried through.
    ///
    /// The caller quotes it back in the
    /// [`client::channel_request::Postgres`](crate::endpoints::agentic_loop::run::client::channel_request::Postgres)
    /// that asks for the container's writes, and that is the whole of
    /// the correlation: a caller with several connections in flight
    /// learns which one it is being asked to feed.
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
    /// quoting one out of a namespace its reader does not share. An
    /// id invented for the connection belongs to neither channel,
    /// which is what makes it readable on both sides.
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
