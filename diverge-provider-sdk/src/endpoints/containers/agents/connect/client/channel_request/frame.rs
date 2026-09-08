//! What a client's channel request frame carries for an agent container connection.

use std::error::Error;
use std::fmt;

use crate::decode::Decode;
use crate::encode::{Encode, Writer};
use crate::shared::containers::{agent_schema, agentic_loop};
use crate::shared::containers::{filetree, postgres, read, write_path};

/// What a caller asks a provider for while an agent container runs.
///
/// A payload leads with one byte saying which, and the rest is that
/// variant's own bytes.
///
/// | tag | asks for |
/// |-----|----------|
/// | `0` | [`Disconnect`](Self::Disconnect) |
/// | `1` | [`Filetree`](Self::Filetree) |
/// | `2` | [`Read`](Self::Read) |
/// | `3` | [`Write`](Self::Write) |
/// | `4` | [`Postgres`](Self::Postgres) |
/// | `5` | [`AgenticLoop`](Self::AgenticLoop) |
/// | `6` | [`AgentSchema`](Self::AgentSchema) |
///
/// The first five are the same in every container scope, in the same
/// order, so a reader of one is a reader of all; what follows is this
/// family's own exchange. All of them but the first reach INTO the
/// container, which is the thing a caller cannot dial: it runs on the
/// provider. That is the whole reason these channels open outward
/// from the client rather than the other way.
#[derive(Debug, Clone, PartialEq)]
pub enum Frame {
    /// Leave the container. Tag `0`.
    ///
    /// # It has no answer, and does not need one
    ///
    /// Nothing comes back on this channel. What comes back is the end
    /// of the SCOPE — a
    /// [`ResponseFinish`](crate::frame::server::ServerFrame::ResponseFinish),
    /// which already means nothing bearing this scope follows on any
    /// channel.
    ///
    /// # What it does not do
    ///
    /// Stop the container. A connector joined something it does not
    /// own, and leaving takes nothing with it: the runner's scope and
    /// every other connector's go on exactly as before. The difference
    /// from dropping the connection is the same as a run's — a
    /// provider cannot tell a deliberate exit from a network that
    /// stopped answering, and this is unambiguous and immediate.
    Disconnect,
    /// The container's filesystem, watched. Tag `1`.
    ///
    /// Carries nothing; the provider answers with a snapshot and then
    /// every change, for as long as the channel lives. See
    /// [`filetree`](crate::shared::containers::filetree).
    Filetree(filetree::request::Request),
    /// One file, read out of the container. Tag `2`.
    ///
    /// See [`read`](crate::shared::containers::read) for why this is
    /// one file and never a directory.
    Read(read::request::Request),
    /// One file, written into the container. Tag `3`.
    ///
    /// Carries no content. The provider answers by opening a channel
    /// of its own asking for it — see
    /// [`write_path`](crate::shared::containers::write_path) for why
    /// it travels that direction, and
    /// [`write_bytes`](crate::shared::containers::write_bytes) for
    /// what comes back.
    Write(write_path::request::Request),
    /// The caller's half of a database connection. Tag `4`.
    ///
    /// Opened once the caller has taken the provider's half, quoting
    /// the same connection; what comes back is everything the
    /// container wrote. See
    /// [`postgres`](crate::shared::containers::postgres) for the pair.
    Postgres(postgres::request::Postgres),
    /// Run the loop. Tag `5`.
    ///
    /// The family's own exchange: a prompt and an agent, and the
    /// loop's chunks back. See
    /// [`agentic_loop`](crate::shared::containers::agentic_loop).
    AgenticLoop(agentic_loop::request::Request),
    /// What the agent may be. Tag `6`.
    ///
    /// Carries nothing; the provider answers with the JSON Schema of
    /// the agent value. See
    /// [`agent_schema`](crate::shared::containers::agent_schema).
    AgentSchema(agent_schema::request::Request),
}

/// Tag for [`Frame::Disconnect`].
const DISCONNECT: u8 = 0;

/// Tag for [`Frame::Filetree`].
const FILETREE: u8 = 1;

/// Tag for [`Frame::Read`].
const READ: u8 = 2;

/// Tag for [`Frame::Write`].
const WRITE: u8 = 3;

/// Tag for [`Frame::Postgres`].
const POSTGRES: u8 = 4;

/// Tag for [`Frame::AgenticLoop`].
const AGENTIC_LOOP: u8 = 5;

/// Tag for [`Frame::AgentSchema`].
const AGENT_SCHEMA: u8 = 6;

impl Encode for Frame {
    /// The ordinary JSON failure, from whichever half has one.
    type Error = serde_json::Error;

    fn encode(&self, out: &mut Writer<'_>) -> Result<(), Self::Error> {
        match self {
            Frame::Disconnect => {
                out.extend_from_slice(&[DISCONNECT]);
                Ok(())
            }
            Frame::Filetree(request) => {
                out.extend_from_slice(&[FILETREE]);
                // Its error is `Infallible`, and an empty match on one
                // is how you say so: there is no value to handle.
                request.encode(out).map_err(|error| match error {})
            }
            Frame::Read(request) => {
                out.extend_from_slice(&[READ]);
                request.encode(out)
            }
            Frame::Write(request) => {
                out.extend_from_slice(&[WRITE]);
                request.encode(out)
            }
            Frame::Postgres(request) => {
                out.extend_from_slice(&[POSTGRES]);
                request.encode(out).map_err(|error| match error {})
            }
            Frame::AgenticLoop(request) => {
                out.extend_from_slice(&[AGENTIC_LOOP]);
                request.encode(out)
            }
            Frame::AgentSchema(request) => {
                out.extend_from_slice(&[AGENT_SCHEMA]);
                // Its error is `Infallible`, and an empty match on one
                // is how you say so: there is no value to handle.
                request.encode(out).map_err(|error| match error {})
            }
        }
    }
}

impl Decode<'_> for Frame {
    /// Several ways to fail, and the parses among them name which.
    type Error = FrameError;

    fn decode(bytes: &[u8]) -> Result<Self, Self::Error> {
        let (tag, rest) = bytes.split_first().ok_or(FrameError::Empty)?;
        match *tag {
            DISCONNECT => Ok(Frame::Disconnect),
            FILETREE => Ok(Frame::Filetree(
                filetree::request::Request::decode(rest)
                    .unwrap_or_else(|error| match error {}),
            )),
            READ => read::request::Request::decode(rest)
                .map(Frame::Read)
                .map_err(FrameError::Read),
            WRITE => write_path::request::Request::decode(rest)
                .map(Frame::Write)
                .map_err(FrameError::Write),
            POSTGRES => postgres::request::Postgres::decode(rest)
                .map(Frame::Postgres)
                .map_err(FrameError::Postgres),
            AGENTIC_LOOP => agentic_loop::request::Request::decode(rest)
                .map(Frame::AgenticLoop)
                .map_err(FrameError::AgenticLoop),
            AGENT_SCHEMA => Ok(Frame::AgentSchema(
                agent_schema::request::Request::decode(rest)
                    .unwrap_or_else(|error| match error {}),
            )),
            tag => Err(FrameError::UnknownTag(tag)),
        }
    }
}

/// An agent container connection channel request that could not be read.
#[derive(Debug)]
pub enum FrameError {
    /// No bytes at all, so not even a tag.
    Empty,
    /// A tag that is none of this frame's.
    UnknownTag(u8),
    /// The read request did not parse.
    Read(serde_json::Error),
    /// The write request did not parse.
    Write(serde_json::Error),
    /// The connection id was not four bytes.
    Postgres(postgres::request::PostgresError),
    /// The loop's request did not parse as JSON.
    AgenticLoop(serde_json::Error),
}

impl fmt::Display for FrameError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FrameError::Empty => {
                f.write_str("agents connect channel request frame is empty")
            }
            FrameError::UnknownTag(tag) => {
                write!(f, "unknown agents connect channel request tag {tag}")
            }
            FrameError::Read(error) => {
                write!(f, "read request did not parse: {error}")
            }
            FrameError::Write(error) => {
                write!(f, "write request did not parse: {error}")
            }
            FrameError::Postgres(error) => write!(f, "{error}"),
            FrameError::AgenticLoop(error) => {
                write!(f, "agentic loop request did not parse: {error}")
            }
        }
    }
}

impl Error for FrameError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            FrameError::Read(error)
            | FrameError::Write(error)
            | FrameError::AgenticLoop(error) => Some(error),
            FrameError::Postgres(error) => Some(error),
            FrameError::Empty | FrameError::UnknownTag(_) => None,
        }
    }
}
