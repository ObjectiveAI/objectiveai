//! The caller asking for a database connection's input.

use std::error::Error;
use std::fmt;

use crate::decode::Decode;
use crate::encode::{Encode, Writer};

/// Send what the container writes to the database.
///
/// The second half of a Postgres connection, and the one that travels
/// the other way. A caller opens a channel with this once it has taken
/// a
/// [`server::channel_request::Postgres`](crate::endpoints::agentic_loop::run::server::channel_request::Postgres),
/// quoting the same [`connection_id`](Self::connection_id).
///
/// What comes back on this channel is everything the CONTAINER
/// writes. What the database says travels the other way, on the
/// channel the provider opened. The plugin endpoint's
/// [twin](crate::endpoints::mcp_plugin::run::client::channel_request::Postgres)
/// is the same exchange.
///
/// # It is the only thing here that does not reach into the container
///
/// The queue verbs are aimed at something the provider is holding on
/// the caller's behalf — the conversation. This one asks for bytes
/// the provider already has and is waiting to hand over. It opens
/// outward for a structural reason rather than a topological one: the
/// writes have to be a RESPONSE stream so that the provider can
/// finish it, and only a channel the caller opened gives the provider
/// something to respond on.
///
/// # What its finish means
///
/// The container's socket ended. Not "there is nothing right now" — a
/// quiet channel is a channel still running — but that no further
/// byte will ever be written by this connection, because the thing
/// writing them is gone.
///
/// Which is the signal the caller acts on by closing its database
/// connection, and it is the whole reason the exchange is shaped this
/// way. A process that dies without sending pgwire's `Terminate` says
/// nothing in its bytes, so without a frame that means this the
/// caller's backend would stay open until the scope ended.
///
/// # Open it, or decline
///
/// A caller that has taken the provider's half owes it one of two
/// things: this, or a finish on the provider's channel. The container
/// wrote its startup message the instant it connected — pgwire is
/// client-first — so the provider is holding bytes until this
/// arrives, and a caller that does neither leaves it holding them for
/// a connection nobody is draining.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Postgres {
    /// The connection being asked about, quoted from the
    /// [`server::channel_request::Postgres`](crate::endpoints::agentic_loop::run::server::channel_request::Postgres)
    /// that opened it.
    ///
    /// Neither side's header can name the other's channels — they are
    /// numbered per SENDER — so the id belongs to the connection
    /// instead of to either channel, which is what lets both ends
    /// read it.
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

/// A Postgres write request that could not be read.
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
                write!(f, "postgres write request is {len} bytes, not 4")
            }
        }
    }
}

impl Error for PostgresError {}
