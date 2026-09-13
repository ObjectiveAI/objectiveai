//! What a client's channel request frame carries for an agent
//! container begin.

use std::error::Error;
use std::fmt;

use crate::decode::Decode;
use crate::encode::{Encode, Writer};
use crate::shared::containers::postgres;
use crate::shared::containers::{enqueue, run_loop};

/// What the server asks the proxy for once an agent container has
/// begun.
///
/// A payload leads with one byte saying which, and the rest is that
/// variant's own bytes.
///
/// | tag | asks for |
/// |-----|----------|
/// | `0` | [`Postgres`](Self::Postgres) |
/// | `1` | [`AgentRun`](Self::AgentRun) |
/// | `2` | [`AgentSchema`](Self::AgentSchema) |
/// | `3` | [`Enqueue`](Self::Enqueue) |
/// | `4` | [`Dequeue`](Self::Dequeue) |
///
/// The first is the same in both begin scopes, so a reader of one is
/// a reader of both; what follows is this family's own exchange. All
/// of them reach INTO the container: the last hop of what a caller
/// opened on the provider.
#[derive(Debug, Clone, PartialEq)]
pub enum Frame {
    /// The server's half of a database connection. Tag `0`.
    ///
    /// Opened once the server has taken the proxy's half, quoting the
    /// same connection; what comes back is everything the container's
    /// driver wrote. See
    /// [`postgres`](crate::shared::containers::postgres) for the pair.
    Postgres(postgres::request::Postgres),
    /// Run a loop. Tag `1`.
    ///
    /// The family's own exchange. Carries the prompt — the agent was
    /// on the request that began the connection, and never changes —
    /// and answers with the loop's chunks. See
    /// [`run_loop`](crate::shared::containers::run_loop).
    AgentRun(run_loop::request::Request),
    /// What the agent may be. Tag `2`.
    ///
    /// Carries nothing — the variant is bare — and the proxy answers
    /// with the JSON Schema of the agent value. See
    /// [`agent_schema`](crate::shared::containers::agent_schema).
    AgentSchema,
    /// A message for the running loop's queue. Tag `3`.
    ///
    /// Answered once — by an
    /// [`enqueue::response::Frame`](crate::shared::containers::enqueue::response::Frame)
    /// naming the message's fate, whenever that is known — and then
    /// the finish. See [`enqueue`](crate::shared::containers::enqueue).
    Enqueue(enqueue::request::Request),
    /// Withdraw every message still waiting in the queue. Tag `4`.
    ///
    /// Carries nothing — the variant is bare. Answered once — by a
    /// [`dequeue::response::Frame`](crate::shared::containers::dequeue::response::Frame)
    /// saying whether the queue held anything — and then the finish.
    /// Each message it withdraws is ALSO answered, on its own enqueue
    /// channel. See [`dequeue`](crate::shared::containers::dequeue).
    Dequeue,
}

/// Tag for [`Frame::Postgres`].
const POSTGRES: u8 = 0;

/// Tag for [`Frame::AgentRun`].
const AGENT_RUN: u8 = 1;

/// Tag for [`Frame::AgentSchema`].
const AGENT_SCHEMA: u8 = 2;

/// Tag for [`Frame::Enqueue`].
const ENQUEUE: u8 = 3;

/// Tag for [`Frame::Dequeue`].
const DEQUEUE: u8 = 4;

impl Encode for Frame {
    /// The ordinary JSON failure, from whichever ask has one.
    type Error = serde_json::Error;

    fn encode(&self, out: &mut Writer<'_>) -> Result<(), Self::Error> {
        match self {
            Frame::Postgres(request) => {
                out.extend_from_slice(&[POSTGRES]);
                // Its error is `Infallible`, and an empty match on one
                // is how you say so: there is no value to handle.
                request.encode(out).map_err(|error| match error {})
            }
            Frame::AgentRun(request) => {
                out.extend_from_slice(&[AGENT_RUN]);
                request.encode(out)
            }
            Frame::AgentSchema => {
                out.extend_from_slice(&[AGENT_SCHEMA]);
                Ok(())
            }
            Frame::Enqueue(request) => {
                out.extend_from_slice(&[ENQUEUE]);
                request.encode(out)
            }
            Frame::Dequeue => {
                out.extend_from_slice(&[DEQUEUE]);
                Ok(())
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
            POSTGRES => postgres::request::Postgres::decode(rest)
                .map(Frame::Postgres)
                .map_err(FrameError::Postgres),
            AGENT_RUN => run_loop::request::Request::decode(rest)
                .map(Frame::AgentRun)
                .map_err(FrameError::AgentRun),
            AGENT_SCHEMA => Ok(Frame::AgentSchema),
            ENQUEUE => enqueue::request::Request::decode(rest)
                .map(Frame::Enqueue)
                .map_err(FrameError::Enqueue),
            DEQUEUE => Ok(Frame::Dequeue),
            tag => Err(FrameError::UnknownTag(tag)),
        }
    }
}

/// An agents begin channel request that could not be read.
#[derive(Debug)]
pub enum FrameError {
    /// No bytes at all, so not even a tag.
    Empty,
    /// A tag that is none of this frame's.
    UnknownTag(u8),
    /// The connection id was not four bytes.
    Postgres(postgres::request::PostgresError),
    /// The loop's prompt did not parse as JSON.
    AgentRun(serde_json::Error),
    /// The enqueued message did not parse as JSON.
    Enqueue(serde_json::Error),
}

impl fmt::Display for FrameError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FrameError::Empty => {
                f.write_str("agents begin channel request frame is empty")
            }
            FrameError::UnknownTag(tag) => {
                write!(f, "unknown agents begin channel request tag {tag}")
            }
            FrameError::Postgres(error) => write!(f, "{error}"),
            FrameError::AgentRun(error) => {
                write!(f, "run loop request did not parse: {error}")
            }
            FrameError::Enqueue(error) => {
                write!(f, "enqueue request did not parse: {error}")
            }
        }
    }
}

impl Error for FrameError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            FrameError::AgentRun(error)
            | FrameError::Enqueue(error) => Some(error),
            FrameError::Postgres(error) => Some(error),
            FrameError::Empty | FrameError::UnknownTag(_) => None,
        }
    }
}
