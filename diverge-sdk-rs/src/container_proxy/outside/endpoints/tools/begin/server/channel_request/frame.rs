//! What a server's channel request frame carries for a tool
//! container begin.

use std::convert::Infallible;
use std::fmt;

use crate::wire::decode::Decode;
use crate::wire::encode::{Encode, Writer};
use crate::shared::containers::{daemon, postgres};

/// What the proxy asks the server for once a tool container
/// has begun.
///
/// A payload leads with one byte saying which, and the rest is that
/// variant's own bytes.
///
/// | tag | asks for |
/// |-----|----------|
/// | `0` | [`Postgres`](Self::Postgres) |
/// | `1` | [`Daemon`](Self::Daemon) |
///
/// Two: everything a tool container asks of the world outside — its
/// database connections, its daemon connection — which the server
/// relays to the caller as the provider protocol's own channel
/// requests. No tool call outward: a tool container makes none, and
/// a proxy serving one answers its program's MCP client with an
/// error; an agent container's begin has those asks and no daemon
/// connection. The fuse asks are not here; each rides the scope of
/// the mount it belongs to.
#[derive(Debug, Clone, PartialEq)]
pub enum Frame {
    /// The proxy's half of a database connection the container's
    /// driver opened. Tag `0`.
    ///
    /// An id the proxy minted. What comes back is everything the
    /// database says; the server opens the other half, or declines by
    /// finishing this one with nothing before it. See
    /// [`postgres`](crate::shared::containers::postgres).
    Postgres(postgres::request::Postgres),
    /// A daemon connection the program opened, by the id the proxy
    /// minted. Tag `1`.
    ///
    /// This half is the proxy's own. What comes back on it is
    /// everything the DAEMON says, as
    /// [`daemon::server::Frame`](crate::shared::containers::daemon::server::Frame)s;
    /// its finish is the connection closed, and an empty finish is the
    /// connection declined. See
    /// [`daemon`](crate::shared::containers::daemon) for the pair.
    Daemon(daemon::request::Daemon),
}

/// Tag for [`Frame::Postgres`].
const POSTGRES: u8 = 0;

/// Tag for [`Frame::Daemon`].
const DAEMON: u8 = 1;

impl Encode for Frame {
    /// [`Infallible`]: both asks are an id, bytes copied.
    type Error = Infallible;

    fn encode(&self, out: &mut Writer<'_>) -> Result<(), Infallible> {
        match self {
            Frame::Postgres(request) => {
                out.extend_from_slice(&[POSTGRES]);
                request.encode(out)
            }
            Frame::Daemon(request) => {
                out.extend_from_slice(&[DAEMON]);
                request.encode(out)
            }
        }
    }
}

impl Decode<'_> for Frame {
    /// Four ways to fail, and each names which ask it was.
    type Error = FrameError;

    fn decode(bytes: &[u8]) -> Result<Self, Self::Error> {
        let (tag, rest) = bytes.split_first().ok_or(FrameError::Empty)?;
        match *tag {
            POSTGRES => postgres::request::Postgres::decode(rest)
                .map(Frame::Postgres)
                .map_err(FrameError::Postgres),
            DAEMON => daemon::request::Daemon::decode(rest)
                .map(Frame::Daemon)
                .map_err(FrameError::Daemon),
            tag => Err(FrameError::UnknownTag(tag)),
        }
    }
}

/// A tools begin channel request that could not be read.
#[derive(Debug)]
pub enum FrameError {
    /// No bytes at all, so not even a tag.
    Empty,
    /// A tag that is none of this frame's two.
    UnknownTag(u8),
    /// The connection id was not four bytes.
    Postgres(postgres::request::PostgresError),
    /// The daemon connection's id was not four bytes.
    Daemon(daemon::request::DaemonError),
}

impl fmt::Display for FrameError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FrameError::Empty => {
                f.write_str("tools begin channel request frame is empty")
            }
            FrameError::UnknownTag(tag) => {
                write!(f, "unknown tools begin channel request tag {tag}")
            }
            FrameError::Postgres(error) => write!(f, "{error}"),
            FrameError::Daemon(error) => write!(f, "{error}"),
        }
    }
}

impl std::error::Error for FrameError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            FrameError::Daemon(error) => Some(error),
            FrameError::Postgres(error) => Some(error),
            FrameError::Empty | FrameError::UnknownTag(_) => None,
        }
    }
}
