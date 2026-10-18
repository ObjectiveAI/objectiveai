//! What a client's channel request frame carries for an agent
//! container begin.

use std::error::Error;
use std::fmt;

use crate::wire::decode::Decode;
use crate::wire::encode::{Encode, Writer};
use crate::shared::containers::secrets::Values;
use crate::shared::containers::{dequeue, enqueue, postgres};

/// What the server asks the proxy for once an agent container has
/// begun.
///
/// A payload leads with one byte saying which, and the rest is that
/// variant's own bytes.
///
/// | tag | asks for |
/// |-----|----------|
/// | `0` | [`Postgres`](Self::Postgres) |
/// | `1` | [`Info`](Self::Info) |
/// | `2` | [`Enqueue`](Self::Enqueue) |
/// | `3` | [`Dequeue`](Self::Dequeue) |
/// | `4` | [`Secrets`](Self::Secrets) |
///
/// The first two are the same in both begin scopes, so a reader of
/// one is a reader of both; what follows is this family's own
/// exchange — no daemon connection among them, since an agent
/// container holds no account and asks the daemon nothing — and
/// then the feed, which both scopes have again under a tag of their
/// own. All of them reach INTO the container: the last hop of what a
/// caller opened on the provider, or of what the caller fed. What
/// comes OUT of the agent — its conversation — is no channel: it
/// rides the begin's own main stream.
#[derive(Debug, Clone, PartialEq)]
pub enum Frame {
    /// The server's half of a database connection. Tag `0`.
    ///
    /// Opened once the server has taken the proxy's half, quoting the
    /// same connection; what comes back is everything the container's
    /// driver wrote. See
    /// [`postgres`](crate::shared::containers::postgres) for the pair.
    Postgres(postgres::request::Postgres),
    /// What the arguments may be. Tag `1`.
    ///
    /// Carries nothing — the variant is bare — and the proxy answers
    /// with the image's DIVERGE.md, parsed — read from the
    /// container's root, the program never asked — or that it has
    /// none. See [`info`](crate::shared::containers::info).
    Info,
    /// A message for the agent. Tag `2`.
    ///
    /// The one way into it: a message with no loop running starts one,
    /// on that message, and a message while one runs joins its queue.
    /// Answered once — by an
    /// [`enqueue::response::Frame`](crate::shared::containers::enqueue::response::Frame)
    /// naming the message's fate, whenever that is known — and then the
    /// finish. What the agent says in reply is the begin's main stream.
    /// See [`enqueue`](crate::shared::containers::enqueue).
    Enqueue(enqueue::request::Request),
    /// Withdraw every message still waiting under a key. Tag `3`.
    ///
    /// Carries the key, as the enqueues gave it. Answered once — by a
    /// [`dequeue::response::Frame`](crate::shared::containers::dequeue::response::Frame)
    /// saying whether anything waited under it — and then the finish.
    /// Each message it withdraws is ALSO answered, on its own enqueue
    /// channel. See [`dequeue`](crate::shared::containers::dequeue).
    Dequeue(dequeue::request::Request),
    /// The secrets' values, fed to the program. Tag `4`.
    ///
    /// Carries the [`Values`] as JSON — every name the program
    /// declared, each to its value — which the proxy hands the
    /// program on `POST /secrets` and holds until that call answers.
    /// Opened by the server once before it sends the run's id and
    /// again at every activation, relaying the caller's own feed.
    /// Answered once — by a
    /// [`feed::response::Frame`](crate::shared::containers::secrets::feed::response::Frame),
    /// `Fed` or the program's refusal — and then the finish; a `Fed`
    /// opens the gate the proxy holds every delivery behind. See
    /// [`secrets`](crate::shared::containers::secrets).
    Secrets(Values),
}

/// Tag for [`Frame::Postgres`].
const POSTGRES: u8 = 0;

/// Tag for [`Frame::Info`].
const INFO: u8 = 1;

/// Tag for [`Frame::Enqueue`].
const ENQUEUE: u8 = 2;

/// Tag for [`Frame::Dequeue`].
const DEQUEUE: u8 = 3;

/// Tag for [`Frame::Secrets`].
const SECRETS: u8 = 4;

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
            Frame::Info => {
                out.extend_from_slice(&[INFO]);
                Ok(())
            }
            Frame::Enqueue(request) => {
                out.extend_from_slice(&[ENQUEUE]);
                request.encode(out)
            }
            Frame::Dequeue(request) => {
                out.extend_from_slice(&[DEQUEUE]);
                request.encode(out)
            }
            Frame::Secrets(values) => {
                out.extend_from_slice(&[SECRETS]);
                serde_json::to_writer(out, values)
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
            INFO => Ok(Frame::Info),
            ENQUEUE => enqueue::request::Request::decode(rest)
                .map(Frame::Enqueue)
                .map_err(FrameError::Enqueue),
            DEQUEUE => dequeue::request::Request::decode(rest)
                .map(Frame::Dequeue)
                .map_err(FrameError::Dequeue),
            SECRETS => serde_json::from_slice(rest)
                .map(Frame::Secrets)
                .map_err(FrameError::Secrets),
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
    /// The enqueued message did not parse as JSON.
    Enqueue(serde_json::Error),
    /// The dequeue's key did not parse as JSON.
    Dequeue(serde_json::Error),
    /// The secrets' values did not parse as JSON. The message is the
    /// parser's, which quotes no value.
    Secrets(serde_json::Error),
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
            FrameError::Enqueue(error) => {
                write!(f, "enqueue request did not parse: {error}")
            }
            FrameError::Dequeue(error) => {
                write!(f, "dequeue request did not parse: {error}")
            }
            FrameError::Secrets(error) => {
                write!(f, "secrets feed did not parse: {error}")
            }
        }
    }
}

impl Error for FrameError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            FrameError::Enqueue(error) | FrameError::Dequeue(error) | FrameError::Secrets(error) => Some(error),
            FrameError::Postgres(error) => Some(error),
            FrameError::Empty | FrameError::UnknownTag(_) => None,
        }
    }
}
