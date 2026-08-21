//! What a client's channel request frame carries for an MCP plugin.

use std::error::Error;
use std::fmt;

use super::Postgres;
use crate::decode::Decode;
use crate::encode::{Encode, Writer};
use crate::shared::http::request::Request;

/// What a caller asks a provider for while a plugin runs.
///
/// A payload leads with one byte saying which — `0` for
/// [`Mcp`](Self::Mcp), `1` for [`Stop`](Self::Stop), `2` for
/// [`Postgres`](Self::Postgres) — and the rest is that variant's own
/// bytes, of which the second has none.
///
/// # Two reach into the container, and one does not
///
/// [`Mcp`](Self::Mcp) and [`Stop`](Self::Stop) are aimed at the thing
/// a caller cannot dial: the container runs on the provider, on a port
/// the provider published to its own loopback and told nobody. That is
/// the whole reason those channels open outward from the client rather
/// than the other way.
///
/// [`Postgres`](Self::Postgres) opens outward for a different reason,
/// and the difference is worth keeping. It asks for bytes the provider
/// is already holding, so topology has nothing to do with it — the
/// writes have to arrive as a RESPONSE stream, because only a
/// responder can finish a channel and the provider needs to be able to
/// say the plugin has gone.
///
/// A [`laboratory run`](crate::endpoints::laboratories::run::client::channel_request::Frame)
/// has five. The three there and missing here are about files — a
/// plugin serves tools rather than holds a filesystem, takes no
/// [`mounts`], and reports no tree — so a read, a write and a transfer
/// have nothing to act on.
///
/// [`mounts`]: crate::endpoints::laboratories::run::client::request::Frame::mounts
#[derive(Debug, Clone, PartialEq)]
pub enum Frame<'a> {
    /// One MCP exchange, toward the container. Tag `0`.
    ///
    /// The provider relays and nothing more. It does not parse
    /// JSON-RPC, does not track sessions, and never reads the
    /// `Mcp-Session-Id` that ties a caller's exchanges together.
    ///
    /// # It is the same relay a laboratory gets, pointed somewhere
    /// else
    ///
    /// A laboratory's MCP server was put there by the provider, so the
    /// provider chose its port. A plugin's arrived with the image and
    /// bound whatever its author chose, which the caller stated as
    /// [`mcp_port`](crate::endpoints::mcp_plugin::run::client::request::Frame::mcp_port).
    /// Both end up as an HTTP request written onto a socket inside the
    /// container. Nothing about the relaying differs; only what it was
    /// aimed at, and that was settled before the container started.
    ///
    /// Which means a wrong `mcp_port` surfaces HERE, as an exchange that
    /// finishes without an answer, rather than when the plugin
    /// started — the container came up fine, and there was never
    /// anything to discover.
    Mcp(Request<'a>),
    /// Stop the container. Tag `1`.
    ///
    /// # It has no answer, and does not need one
    ///
    /// Nothing comes back on this channel. What comes back is the end
    /// of the SCOPE — a
    /// [`ResponseFinish`](crate::frame::server::ServerFrame::ResponseFinish),
    /// which already means nothing bearing this scope follows on any
    /// channel. Finishing this one first would be a smaller way of
    /// saying the same thing, moments earlier.
    ///
    /// # What it adds over closing the connection
    ///
    /// The scope IS the plugin's life, so dropping the connection
    /// stops it too. The difference is that a provider cannot tell a
    /// deliberate exit from a network that stopped answering, and has
    /// to wait to find out. This is unambiguous and immediate: a
    /// caller that says so is not gone, it is finished.
    ///
    /// # It ends nobody else
    ///
    /// Unlike a
    /// [`laboratory's`](crate::endpoints::laboratories::run::client::channel_request::Frame::Stop),
    /// which takes every connector down with it. A plugin has no
    /// connectors and no
    /// [`id`](crate::endpoints::laboratories::run::server::response::Frame::Id)
    /// by which one could have arrived, so the caller that created it
    /// is the only party to the container's existence and stopping it
    /// concerns nobody else.
    ///
    /// # What it does to exchanges in flight
    ///
    /// Ends them, unanswered. A caller with MCP channels still open
    /// when it sends this will see them finish without heads, because
    /// the container they were aimed at is gone. Waiting for them
    /// first is the caller's to do, and nothing here does it on the
    /// caller's behalf — a provider that tried would be guessing which
    /// of a caller's outstanding calls it still wanted.
    Stop,
    /// The plugin's half of a database connection. Tag `2`.
    ///
    /// Sent in answer to a
    /// [`server::channel_request::Frame::Postgres`](crate::endpoints::mcp_plugin::run::server::channel_request::Frame::Postgres),
    /// quoting the connection it names. What comes back is everything
    /// the plugin writes; the finish says the plugin's socket ended.
    ///
    /// See [`Postgres`] for why a connection takes two channels and
    /// what a caller owes the provider once it has taken the first.
    Postgres(Postgres),
}

/// Tag for [`Frame::Mcp`].
const MCP: u8 = 0;

/// Tag for [`Frame::Stop`].
const STOP: u8 = 1;

/// Tag for [`Frame::Postgres`].
const POSTGRES: u8 = 2;

impl Encode for Frame<'_> {
    /// The ordinary JSON failure, from the only variant that has one.
    /// A stop carries nothing and a connection id is four known bytes.
    type Error = serde_json::Error;

    fn encode(&self, out: &mut Writer<'_>) -> Result<(), Self::Error> {
        match self {
            Frame::Mcp(request) => {
                out.extend_from_slice(&[MCP]);
                request.encode(out)
            }
            Frame::Stop => {
                out.extend_from_slice(&[STOP]);
                Ok(())
            }
            Frame::Postgres(postgres) => {
                out.extend_from_slice(&[POSTGRES]);
                postgres
                    .encode(out)
                    .unwrap_or_else(|error| match error {});
                Ok(())
            }
        }
    }
}

impl<'a> Decode<'a> for Frame<'a> {
    /// Four ways to fail, and only one of them is JSON.
    type Error = FrameError;

    fn decode(bytes: &'a [u8]) -> Result<Self, Self::Error> {
        let (tag, rest) = bytes.split_first().ok_or(FrameError::Empty)?;
        match *tag {
            MCP => Request::decode(rest).map(Frame::Mcp).map_err(FrameError::Mcp),
            STOP => Ok(Frame::Stop),
            POSTGRES => Postgres::decode(rest)
                .map(Frame::Postgres)
                .map_err(FrameError::Postgres),
            tag => Err(FrameError::UnknownTag(tag)),
        }
    }
}

/// An MCP plugin channel request that could not be read.
#[derive(Debug)]
pub enum FrameError {
    /// No bytes at all, so not even a tag.
    Empty,
    /// A tag that is none of this frame's three.
    UnknownTag(u8),
    /// The MCP request did not parse.
    Mcp(serde_json::Error),
    /// The write request was not a connection id.
    Postgres(super::postgres::PostgresError),
}

impl fmt::Display for FrameError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FrameError::Empty => {
                f.write_str("mcp plugin channel request frame is empty")
            }
            FrameError::UnknownTag(tag) => {
                write!(f, "unknown mcp plugin channel request tag {tag}")
            }
            FrameError::Mcp(error) => {
                write!(f, "mcp request did not parse: {error}")
            }
            FrameError::Postgres(error) => {
                write!(f, "postgres write request did not parse: {error}")
            }
        }
    }
}

impl Error for FrameError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            FrameError::Mcp(error) => Some(error),
            FrameError::Postgres(error) => Some(error),
            FrameError::Empty | FrameError::UnknownTag(_) => None,
        }
    }
}
