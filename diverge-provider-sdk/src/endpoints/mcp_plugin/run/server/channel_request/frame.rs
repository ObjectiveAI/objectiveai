//! What a server's channel request frame carries for an MCP plugin.

use std::error::Error;
use std::fmt;

use super::Postgres;
use crate::decode::Decode;
use crate::encode::{Encode, Writer};
use crate::shared::http::request::Request;

/// What a provider asks a caller for while a plugin runs.
///
/// A payload leads with one byte saying which — `0` for
/// [`Oci`](Self::Oci), `1` for [`Postgres`](Self::Postgres), `2` for
/// [`Command`](Self::Command) — and the rest is that variant's own
/// bytes.
///
/// All three are the same ask in different clothes: something the
/// provider cannot reach. The image lives with the caller, so does the
/// database, and so does the daemon that runs commands. The container
/// runs beside the provider, so the provider opens a channel and the
/// caller splices the far end into the real thing.
///
/// A [`laboratory run`](crate::endpoints::laboratories::run::server::channel_request::Frame)
/// asks for an image the same way and for two other things that have
/// nothing to act on here: an authorization gates a connector a plugin
/// does not have, and a write asks for content a plugin never takes.
///
/// # Why the three are shaped differently
///
/// [`Oci`](Self::Oci) is a structured exchange, [`Postgres`](Self::Postgres)
/// names a connection, and [`Command`](Self::Command) is bytes. That is
/// not a preference — it is what each thing IS.
///
/// A registry pull is a series of discrete requests, each with a
/// method, a path and a status, and this specification RELIES on those
/// semantics: `404` means a blob is absent, `206` resumes, `HEAD`
/// probes. Reducing it to bytes would throw away meaning the protocol
/// is built on.
///
/// A Postgres session is a long-lived socket carrying a conversation
/// with no natural top-level unit — so it is not an exchange at all,
/// and this frame does not try to make it one. It opens HALF a
/// connection, and the caller opens the other half. See
/// [`Postgres`] for why a socket has to be two channels.
///
/// A command is neither. It is one ask and a stream of answers, framed
/// by the channel itself — and its CONTENTS belong to a vocabulary
/// this specification does not own.
#[derive(Debug, Clone, PartialEq)]
pub enum Frame<'a> {
    /// One request against the caller's registry. Tag `0`.
    ///
    /// Opened only for an
    /// [`Image::Client`](crate::shared::container::request::Image::Client)
    /// plugin, and opened by the container RUNTIME's appetite rather
    /// than the provider's: the provider serves a registry endpoint,
    /// the runtime pulls from it, and every request the runtime makes
    /// that the provider cannot answer from what it holds becomes one
    /// of these.
    ///
    /// The provider understands none of it. It does not parse the
    /// manifest to find layers, does not diff digests against a store
    /// of its own, and does not decide what a blob is. Which means a
    /// runtime's cache is the only cache, its dedup is the only dedup,
    /// and `Range` resumes and `HEAD` probes work because nothing here
    /// had to be taught about them.
    ///
    /// The repository segment of the path names the scope, so one
    /// endpoint serves every run happening at once — plugins and
    /// laboratories alike — and a request routes itself without a
    /// provider keeping state between them.
    Oci(Request<'a>),
    /// One database connection, opened toward the caller. Tag `1`.
    ///
    /// Carries no bytes. It names a connection and asks the caller to
    /// dial its database; what the database SAYS comes back on this
    /// channel, and what the plugin WRITES arrives on a second channel
    /// the caller opens quoting the same id — see [`Postgres`].
    ///
    /// # Why a plugin, and not the agent
    ///
    /// Because a plugin is what needs a database. An agent talks to
    /// its tools; a tool is what keeps something. So the tunnel ends
    /// where the tool runs, and the loop that called it never sees a
    /// connection it has no query to send down.
    ///
    /// # Never parsed
    ///
    /// Which is what lets TLS negotiation and every protocol extension
    /// cross untouched. A conduit that understood pgwire would have to
    /// keep up with it; one that does not is finished being written.
    ///
    /// It is why the bytes are never framed as messages either: a
    /// pgwire message larger than one frame simply spans several, and
    /// both ends reassemble as they would from a socket.
    Postgres(Postgres),
    /// One Diverge command, toward the caller. Tag `2`.
    ///
    /// A plugin has no CLI binary in its container and no daemon it is
    /// allowed to dial, so a command it wants run has to be run by
    /// somebody who can. That is the caller. The plugin asks, the
    /// provider relays, the caller executes, and the answers come back
    /// on this channel as
    /// [`command`](crate::endpoints::mcp_plugin::run::client::channel_response::command)
    /// frames.
    ///
    /// # One ask, one answer
    ///
    /// One of these opens the channel and nothing follows it in this
    /// direction. What comes back is the answer to it: a head, then as
    /// much body as there turns out to be, then a finish.
    ///
    /// Which is why it is a channel rather than a field on some
    /// existing exchange — the channel already means "one thing asked,
    /// answers until finished", and that is exactly an HTTP exchange's
    /// shape.
    ///
    /// # HTTP, like [`Oci`](Self::Oci), and not for the same reason
    ///
    /// A registry request is HTTP because a registry speaks HTTP. A
    /// command is HTTP because it needed SOME envelope, and this is the
    /// one the endpoint already carries: a method, a path, headers and
    /// a body, answered with a status, headers and a body.
    ///
    /// What that buys is everything an envelope is for, none of it
    /// invented here. A command that fails says so in a status rather
    /// than in an item a reader has to recognise as a failure. A
    /// command that answers once and a command that answers a thousand
    /// times are one exchange with different bodies, where before every
    /// command was a stream because one of them had to be. And a header
    /// is somewhere to put what is about the asking rather than about
    /// the command.
    ///
    /// # A shape, still not a vocabulary
    ///
    /// The path is where a command names itself, and this specification
    /// does not say what may go there. That belongs to the CLI, which
    /// gains subcommands on its own schedule, and a protocol that
    /// enumerated them would be revised every time one appeared —
    /// coupling the shape of the wire to a surface that moves faster
    /// than it.
    ///
    /// The body is opaque for the same reason, in both directions.
    ///
    /// So a provider relays and never reads. It cannot tell one command
    /// from another, which also means it cannot decide it disapproves
    /// of one — what a plugin may ask for is settled between the plugin
    /// and the caller, using the
    /// [`identity`](crate::endpoints::mcp_plugin::run::client::request::Frame::identity)
    /// the caller supplied.
    Command(Request<'a>),
}

/// Tag for [`Frame::Oci`].
const OCI: u8 = 0;

/// Tag for [`Frame::Postgres`].
const POSTGRES: u8 = 1;

/// Tag for [`Frame::Command`].
const COMMAND: u8 = 2;

impl Encode for Frame<'_> {
    /// The ordinary JSON failure, from the two variants that carry a
    /// request. A connection id is four known bytes and cannot
    /// contribute one.
    type Error = serde_json::Error;

    fn encode(&self, out: &mut Writer<'_>) -> Result<(), Self::Error> {
        match self {
            Frame::Oci(request) => {
                out.extend_from_slice(&[OCI]);
                request.encode(out)
            }
            Frame::Postgres(postgres) => {
                out.extend_from_slice(&[POSTGRES]);
                postgres
                    .encode(out)
                    .unwrap_or_else(|error| match error {});
                Ok(())
            }
            Frame::Command(request) => {
                out.extend_from_slice(&[COMMAND]);
                request.encode(out)
            }
        }
    }
}

impl<'a> Decode<'a> for Frame<'a> {
    /// Four ways to fail, and two of them are parses. A command is the
    /// only one of the three payloads with nothing to get wrong.
    type Error = FrameError;

    fn decode(bytes: &'a [u8]) -> Result<Self, Self::Error> {
        let (tag, rest) = bytes.split_first().ok_or(FrameError::Empty)?;
        match *tag {
            OCI => Request::decode(rest).map(Frame::Oci).map_err(FrameError::Oci),
            POSTGRES => Postgres::decode(rest)
                .map(Frame::Postgres)
                .map_err(FrameError::Postgres),
            COMMAND => Request::decode(rest)
                .map(Frame::Command)
                .map_err(FrameError::Command),
            tag => Err(FrameError::UnknownTag(tag)),
        }
    }
}

/// An MCP plugin channel request that could not be read.
#[derive(Debug)]
pub enum FrameError {
    /// No bytes at all, so not even a tag.
    ///
    /// Distinct from a zero-length write, which is a tag byte followed
    /// by nothing and is ordinary on a socket.
    Empty,
    /// A tag that is none of this frame's three.
    UnknownTag(u8),
    /// The registry request did not parse.
    Oci(serde_json::Error),
    /// The connection request was not a connection id.
    Postgres(super::postgres::PostgresError),
    /// The command request did not parse.
    Command(serde_json::Error),
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
            FrameError::Oci(error) => {
                write!(f, "registry request did not parse: {error}")
            }
            FrameError::Postgres(error) => {
                write!(f, "postgres connection request did not parse: {error}")
            }
            FrameError::Command(error) => {
                write!(f, "command request did not parse: {error}")
            }
        }
    }
}

impl Error for FrameError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            FrameError::Oci(error) => Some(error),
            FrameError::Postgres(error) => Some(error),
            FrameError::Command(error) => Some(error),
            FrameError::Empty | FrameError::UnknownTag(_) => None,
        }
    }
}
