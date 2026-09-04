//! What a server's channel request frame carries for an MCP plugin.

use std::convert::Infallible;
use std::error::Error;
use std::fmt;

use super::Postgres;
use crate::decode::Decode;
use crate::encode::{Encode, Writer};
use crate::shared::oci;

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
/// [`Oci`](Self::Oci) is a request forwarded whole,
/// [`Postgres`](Self::Postgres) names a connection, and
/// [`Command`](Self::Command) is an ask framed by its channel. That is
/// not a preference — it is what each thing IS.
///
/// A registry pull is a series of discrete requests whose semantics —
/// `404` for an absent blob, `206` resuming, `HEAD` probing — belong to
/// the registry and the runtime speaking to it. Forwarding each one
/// verbatim is what keeps those semantics intact: nothing between the
/// two reads a header, so nothing between the two can get one wrong.
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
    Oci(oci::request::Request<'a>),
    /// One database connection, opened toward the caller. Tag `1`.
    ///
    /// Carries no bytes. It names a connection and asks the caller to
    /// dial its database; what the database SAYS comes back on this
    /// channel, and what the plugin WRITES arrives on a second channel
    /// the caller opens quoting the same id — see [`Postgres`].
    ///
    /// # The loop carries it too
    ///
    /// For a long time only a plugin did, on the grounds that a plugin
    /// is what needs a database — an agent talks to its tools, and a
    /// tool is what keeps something. An upstream whose own state is
    /// rows ended that, and the
    /// [agentic loop](crate::endpoints::agentic_loop::run::server::channel_request::Frame::Postgres)
    /// now opens the same pair, at a fixed port instead of a declared
    /// one.
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
    /// # One ask, then a stream
    ///
    /// One of these opens the channel and nothing follows it in this
    /// direction. The answer is as many response frames as the command
    /// produces items, then a finish — so a command yielding a thousand
    /// rows delivers them as they come rather than as one document
    /// assembled first.
    ///
    /// Which is why it is a channel rather than a field on some
    /// existing exchange: the channel already means "one thing asked,
    /// answers until finished", and that is exactly a command's shape.
    ///
    /// # Opaque, and for a different reason than Postgres
    ///
    /// Postgres is opaque because parsing it would mean tracking a wire
    /// protocol. This is opaque because the command vocabulary is not
    /// this specification's to define. It belongs to the CLI, which
    /// gains subcommands on its own schedule, and a protocol that named
    /// them would have to be revised every time one appeared — coupling
    /// the shape of the wire to a surface that moves faster than it.
    ///
    /// So a provider relays and never reads. It cannot tell one command
    /// from another, which also means it cannot decide it disapproves
    /// of one — what a plugin may ask for is settled between the plugin
    /// and the caller, using the
    /// [`identity`](crate::endpoints::mcp_plugin::run::client::request::Frame::identity)
    /// the caller supplied.
    ///
    /// # Bytes, for a different reason than the registry's
    ///
    /// [`Oci`](Self::Oci) carries bytes because a registry SPEAKS a
    /// protocol and forwarding it untouched is what keeps it working.
    /// Nothing speaks a command but the CLI, and the CLI is on the
    /// other side of this relay — so there is no protocol here to be
    /// faithful to, and an envelope would be one this specification
    /// invented and then had to justify.
    ///
    /// What a command IS lives inside these bytes, and this end never
    /// looks.
    Command(&'a [u8]),
}

/// Tag for [`Frame::Oci`].
const OCI: u8 = 0;

/// Tag for [`Frame::Postgres`].
const POSTGRES: u8 = 1;

/// Tag for [`Frame::Command`].
const COMMAND: u8 = 2;

impl Encode for Frame<'_> {
    /// [`Infallible`], all three being bytes. A connection id is four
    /// known bytes and a command is bytes copied, and a registry
    /// request is now bytes copied too.
    ///
    /// It was the registry request's error, which was JSON's. A
    /// [`Request`](oci::request::Request) carries what the runtime
    /// wrote, verbatim, so the last thing here with anything to get
    /// wrong stopped having it.
    type Error = Infallible;

    fn encode(&self, out: &mut Writer<'_>) -> Result<(), Infallible> {
        // Each arm discharges its own, and an empty match on an
        // [`Infallible`] is how you say there is no value to handle.
        match self {
            Frame::Oci(request) => {
                out.extend_from_slice(&[OCI]);
                request.encode(out).unwrap_or_else(|error| match error {});
            }
            Frame::Postgres(postgres) => {
                out.extend_from_slice(&[POSTGRES]);
                postgres
                    .encode(out)
                    .unwrap_or_else(|error| match error {});
            }
            Frame::Command(bytes) => {
                out.extend_from_slice(&[COMMAND]);
                out.extend_from_slice(bytes);
            }
        }
        Ok(())
    }
}

impl<'a> Decode<'a> for Frame<'a> {
    /// Four ways to fail, and two of them are parses. A command is the
    /// only one of the three payloads with nothing to get wrong.
    type Error = FrameError;

    fn decode(bytes: &'a [u8]) -> Result<Self, Self::Error> {
        let (tag, rest) = bytes.split_first().ok_or(FrameError::Empty)?;
        match *tag {
            OCI => Ok(Frame::Oci(
                oci::request::Request::decode(rest)
                    .unwrap_or_else(|error| match error {}),
            )),
            POSTGRES => Postgres::decode(rest)
                .map(Frame::Postgres)
                .map_err(FrameError::Postgres),
            COMMAND => Ok(Frame::Command(rest)),
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
    /// The connection request was not a connection id.
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
            FrameError::Postgres(error) => {
                write!(f, "postgres connection request did not parse: {error}")
            }
        }
    }
}

impl Error for FrameError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            FrameError::Postgres(error) => Some(error),
            FrameError::Empty | FrameError::UnknownTag(_) => None,
        }
    }
}
