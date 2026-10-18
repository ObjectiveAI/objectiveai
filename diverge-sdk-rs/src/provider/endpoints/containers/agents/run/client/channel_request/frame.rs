//! What a client's channel request frame carries for an agent container
//! run.

use std::error::Error;
use std::fmt;

use crate::wire::decode::Decode;
use crate::wire::encode::{Encode, Writer};
use crate::shared::containers::secrets::Values;
use crate::shared::containers::{dequeue, enqueue};
use crate::shared::containers::{fuse_mount, postgres, read, transfer, write_path};

/// What a caller asks a provider for while an agent container runs.
///
/// A payload leads with one byte saying which, and the rest is that
/// variant's own bytes.
///
/// | tag | asks for |
/// |-----|----------|
/// | `0` | [`Stop`](Self::Stop) |
/// | `1` | [`Filetree`](Self::Filetree) |
/// | `2` | [`Read`](Self::Read) |
/// | `3` | [`Write`](Self::Write) |
/// | `4` | [`Transfer`](Self::Transfer) |
/// | `5` | [`Postgres`](Self::Postgres) |
/// | `6` | [`Schema`](Self::Schema) |
/// | `7` | [`Enqueue`](Self::Enqueue) |
/// | `8` | [`Dequeue`](Self::Dequeue) |
/// | `9` | [`FuseMount`](Self::FuseMount) |
/// | `10` | [`Secrets`](Self::Secrets) |
///
/// The first seven are the same in every container scope, in the same
/// order, so a reader of one is a reader of all; what follows is this
/// family's own exchange — no daemon connection among them, since an
/// agent container holds no account and asks the daemon nothing —
/// and then the mount and the feed, which every scope has again, each
/// under a tag of its own. All of them but the first reach INTO the
/// container, which is the thing a caller cannot dial: it runs on the
/// provider. That is the whole reason these channels open outward
/// from the client rather than the other way. What comes OUT of the
/// agent — its conversation — is no channel: it rides the scope's own
/// main stream.
#[derive(Debug, Clone, PartialEq)]
pub enum Frame {
    /// Stop the container. Tag `0`.
    ///
    /// # It has no answer, and does not need one
    ///
    /// Nothing comes back on this channel. What comes back is the end
    /// of the SCOPE — a
    /// [`ResponseFinish`](crate::wire::frame::server::ServerFrame::ResponseFinish),
    /// which already means nothing bearing this scope follows on any
    /// channel. Finishing this one first would be a smaller way of
    /// saying the same thing, moments earlier.
    ///
    /// # What it adds over closing the connection
    ///
    /// The scope IS the container's life, so dropping the connection
    /// stops it too. The difference is that a provider cannot tell a
    /// deliberate exit from a network that stopped answering, and has
    /// to wait to find out. This is unambiguous and immediate: a
    /// caller that says so is not gone, it is finished.
    ///
    Stop,
    /// The container's filesystem, watched. Tag `1`.
    ///
    /// Carries nothing — the variant is bare — and the provider answers
    /// with a snapshot and then every change, for as long as the
    /// channel lives. See
    /// [`filetree`](crate::shared::containers::filetree).
    Filetree,
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
    /// One file, copied into another container. Tag `4`.
    ///
    /// The other container by its id, and the caller must be running
    /// it. The provider reads the file out of this
    /// container and writes it into that one on its own connections
    /// to the two proxies, and nothing of the file comes back here —
    /// one answer does. See
    /// [`transfer`](crate::shared::containers::transfer) for the rule.
    Transfer(transfer::request::Request),
    /// The caller's half of a database connection. Tag `5`.
    ///
    /// Opened once the caller has taken the provider's half, quoting
    /// the same connection; what comes back is everything the
    /// container wrote. See
    /// [`postgres`](crate::shared::containers::postgres) for the pair.
    Postgres(postgres::request::Postgres),
    /// What the arguments may be. Tag `6`.
    ///
    /// Carries nothing — the variant is bare — and the provider answers
    /// with the JSON Schema of the container's arguments. See
    /// [`schema`](crate::shared::containers::schema).
    Schema,
    /// A message for the agent. Tag `7`.
    ///
    /// The one way into it: a message with no loop running starts one,
    /// on that message, and a message while one runs joins its queue.
    /// Answered once — by an
    /// [`enqueue::response::Frame`](crate::shared::containers::enqueue::response::Frame)
    /// naming the message's fate, whenever that is known — and then the
    /// finish. What the agent says in reply is the scope's main stream.
    /// See [`enqueue`](crate::shared::containers::enqueue).
    Enqueue(enqueue::request::Request),
    /// Withdraw every message still waiting under a key. Tag `8`.
    ///
    /// Carries the key, as the enqueues gave it. Answered once — by a
    /// [`dequeue::response::Frame`](crate::shared::containers::dequeue::response::Frame)
    /// saying whether anything waited under it — and then the finish.
    /// Each message it withdraws is ALSO answered, on its own enqueue
    /// channel. See [`dequeue`](crate::shared::containers::dequeue).
    Dequeue(dequeue::request::Request),
    /// A FUSE mount made while the container runs. Tag `9`.
    ///
    /// The caller serves one more file or directory into the
    /// container at a path, under an id of its own, as the mounts of
    /// the run's request are served. Answered once — by a
    /// [`fuse_mount::response::Frame`](crate::shared::containers::fuse_mount::response::Frame)
    /// saying the mount is made, or why not — and then the finish;
    /// the answer precedes every ask of the id. See
    /// [`fuse_mount`](crate::shared::containers::fuse_mount).
    FuseMount(fuse_mount::request::Request),
    /// The secrets' values, fed to the program at an activation. Tag
    /// `10`.
    ///
    /// Carries the [`Values`] as JSON — every name the program
    /// declared, each to its value, a `fixed` one read and a
    /// `rotating` one locked — which the provider relays to the proxy,
    /// and the proxy hands the program before it delivers anything.
    /// Opened by the caller when an idle container is about to
    /// receive work and holds no values: after the registration's
    /// own feed, that is after every `Inactive`. Answered once — by a
    /// [`feed::response::Frame`](crate::shared::containers::secrets::feed::response::Frame),
    /// `Fed` or the program's refusal — and then the finish. See
    /// [`secrets`](crate::shared::containers::secrets).
    Secrets(Values),
}

/// Tag for [`Frame::Stop`].
const STOP: u8 = 0;

/// Tag for [`Frame::Filetree`].
const FILETREE: u8 = 1;

/// Tag for [`Frame::Read`].
const READ: u8 = 2;

/// Tag for [`Frame::Write`].
const WRITE: u8 = 3;

/// Tag for [`Frame::Transfer`].
const TRANSFER: u8 = 4;

/// Tag for [`Frame::Postgres`].
const POSTGRES: u8 = 5;

/// Tag for [`Frame::Schema`].
const SCHEMA: u8 = 6;

/// Tag for [`Frame::Enqueue`].
const ENQUEUE: u8 = 7;

/// Tag for [`Frame::Dequeue`].
const DEQUEUE: u8 = 8;

/// Tag for [`Frame::FuseMount`].
const FUSE_MOUNT: u8 = 9;

/// Tag for [`Frame::Secrets`].
const SECRETS: u8 = 10;

impl Encode for Frame {
    /// The ordinary JSON failure, from whichever half has one.
    type Error = serde_json::Error;

    fn encode(&self, out: &mut Writer<'_>) -> Result<(), Self::Error> {
        match self {
            Frame::Stop => {
                out.extend_from_slice(&[STOP]);
                Ok(())
            }
            Frame::Filetree => {
                out.extend_from_slice(&[FILETREE]);
                Ok(())
            }
            Frame::Read(request) => {
                out.extend_from_slice(&[READ]);
                request.encode(out)
            }
            Frame::Write(request) => {
                out.extend_from_slice(&[WRITE]);
                request.encode(out)
            }
            Frame::Transfer(request) => {
                out.extend_from_slice(&[TRANSFER]);
                request.encode(out)
            }
            Frame::Postgres(request) => {
                out.extend_from_slice(&[POSTGRES]);
                request.encode(out).map_err(|error| match error {})
            }
            Frame::Schema => {
                out.extend_from_slice(&[SCHEMA]);
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
            Frame::FuseMount(request) => {
                out.extend_from_slice(&[FUSE_MOUNT]);
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
            STOP => Ok(Frame::Stop),
            FILETREE => Ok(Frame::Filetree),
            READ => read::request::Request::decode(rest)
                .map(Frame::Read)
                .map_err(FrameError::Read),
            WRITE => write_path::request::Request::decode(rest)
                .map(Frame::Write)
                .map_err(FrameError::Write),
            TRANSFER => transfer::request::Request::decode(rest)
                .map(Frame::Transfer)
                .map_err(FrameError::Transfer),
            POSTGRES => postgres::request::Postgres::decode(rest)
                .map(Frame::Postgres)
                .map_err(FrameError::Postgres),
            SCHEMA => Ok(Frame::Schema),
            ENQUEUE => enqueue::request::Request::decode(rest)
                .map(Frame::Enqueue)
                .map_err(FrameError::Enqueue),
            DEQUEUE => dequeue::request::Request::decode(rest)
                .map(Frame::Dequeue)
                .map_err(FrameError::Dequeue),
            FUSE_MOUNT => fuse_mount::request::Request::decode(rest)
                .map(Frame::FuseMount)
                .map_err(FrameError::FuseMount),
            SECRETS => serde_json::from_slice(rest)
                .map(Frame::Secrets)
                .map_err(FrameError::Secrets),
            tag => Err(FrameError::UnknownTag(tag)),
        }
    }
}

/// An agent container run channel request that could not be read.
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
    /// The transfer request did not parse.
    Transfer(serde_json::Error),
    /// The connection id was not four bytes.
    Postgres(postgres::request::PostgresError),
    /// The enqueued message did not parse as JSON.
    Enqueue(serde_json::Error),
    /// The dequeue's key did not parse as JSON.
    Dequeue(serde_json::Error),
    /// The mount request did not parse.
    FuseMount(serde_json::Error),
    /// The secrets' values did not parse as JSON. The message is the
    /// parser's, which quotes no value.
    Secrets(serde_json::Error),
}

impl fmt::Display for FrameError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FrameError::Empty => {
                f.write_str("agents run channel request frame is empty")
            }
            FrameError::UnknownTag(tag) => {
                write!(f, "unknown agents run channel request tag {tag}")
            }
            FrameError::Read(error) => {
                write!(f, "read request did not parse: {error}")
            }
            FrameError::Write(error) => {
                write!(f, "write request did not parse: {error}")
            }
            FrameError::Transfer(error) => {
                write!(f, "transfer request did not parse: {error}")
            }
            FrameError::Postgres(error) => write!(f, "{error}"),
            FrameError::Enqueue(error) => {
                write!(f, "enqueue request did not parse: {error}")
            }
            FrameError::Dequeue(error) => {
                write!(f, "dequeue request did not parse: {error}")
            }
            FrameError::FuseMount(error) => {
                write!(f, "fuse mount request did not parse: {error}")
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
            FrameError::Read(error)
            | FrameError::Write(error)
            | FrameError::Transfer(error)
            | FrameError::Enqueue(error)
            | FrameError::Dequeue(error)
            | FrameError::FuseMount(error)
            | FrameError::Secrets(error) => Some(error),
            FrameError::Postgres(error) => Some(error),
            FrameError::Empty | FrameError::UnknownTag(_) => None,
        }
    }
}
