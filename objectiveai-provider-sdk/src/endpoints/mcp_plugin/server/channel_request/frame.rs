//! What a server's channel request frame carries for an MCP plugin.

use std::error::Error;
use std::fmt;

use crate::decode::Decode;
use crate::encode::{Encode, Writer};
use crate::shared::http::request::Request;

/// What a provider asks a caller for while a plugin runs.
///
/// A payload leads with one byte saying which — `0` for
/// [`Oci`](Self::Oci), `1` for [`Postgres`](Self::Postgres) — and the
/// rest is that variant's own bytes.
///
/// Both are the same ask in different clothes: something the provider
/// cannot reach. The image lives with the caller, and so does the
/// database. The container runs beside the provider, so the provider
/// opens a channel and the caller splices the far end into the real
/// thing.
///
/// A [`laboratory creation`](crate::endpoints::laboratories::create::server::channel_request::Frame)
/// asks for an image the same way and for two other things that have
/// nothing to act on here: an authorization gates a connector a plugin
/// does not have, and a write asks for content a plugin never takes.
///
/// # Why the two are shaped differently
///
/// [`Postgres`](Self::Postgres) is a byte stream and [`Oci`](Self::Oci)
/// is a structured exchange, because pgwire really is a CONNECTION and
/// a registry pull really is not.
///
/// A Postgres session is a long-lived socket carrying a conversation
/// with no natural top-level unit, so successive request frames on one
/// channel are successive writes, and a message larger than one frame
/// simply spans several. A registry pull is a series of discrete
/// requests, each with a method, a path and a status, and each
/// answered on its own.
#[derive(Debug, Clone, PartialEq)]
pub enum Frame<'a> {
    /// One request against the caller's registry. Tag `0`.
    ///
    /// Opened only for an
    /// [`ImageType::Client`](crate::shared::container::request::ImageType::Client)
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
    /// endpoint serves every creation happening at once — plugins and
    /// laboratories alike — and a request routes itself without a
    /// provider keeping state between them.
    Oci(Request<'a>),
    /// Postgres bytes, toward the caller's database. Tag `1`.
    ///
    /// Opaque, and a stream — this is a socket, and successive frames
    /// on the channel are successive writes.
    ///
    /// # Why a plugin, and not the agent
    ///
    /// Because a plugin is what needs a database. An agent talks to
    /// its tools; a tool is what keeps something. So the tunnel ends
    /// where the tool runs, and the loop that called it never sees a
    /// connection it has no query to send down.
    ///
    /// # It is opened, not offered
    ///
    /// A provider opens this because something inside the container
    /// dialled the conduit it was given. A plugin that never connects
    /// means this channel never exists — which is what makes an
    /// opted-out plugin cost nothing rather than cost an idle tunnel.
    ///
    /// # Never parsed
    ///
    /// Which is what lets TLS negotiation and every protocol extension
    /// cross untouched. A conduit that understood pgwire would have to
    /// keep up with it; one that does not is finished being written.
    Postgres(&'a [u8]),
}

/// Tag for [`Frame::Oci`].
const OCI: u8 = 0;

/// Tag for [`Frame::Postgres`].
const POSTGRES: u8 = 1;

impl Encode for Frame<'_> {
    /// The registry request's error, since the other variant has none.
    /// Postgres bytes are copied, and copying cannot fail — so the
    /// union of the two is just what a registry request can do wrong.
    type Error = serde_json::Error;

    fn encode(&self, out: &mut Writer<'_>) -> Result<(), Self::Error> {
        match self {
            Frame::Oci(request) => {
                out.extend_from_slice(&[OCI]);
                request.encode(out)
            }
            Frame::Postgres(bytes) => {
                out.extend_from_slice(&[POSTGRES]);
                out.extend_from_slice(bytes);
                Ok(())
            }
        }
    }
}

impl<'a> Decode<'a> for Frame<'a> {
    /// Three ways to fail, and only one of them is a parse.
    type Error = FrameError;

    fn decode(bytes: &'a [u8]) -> Result<Self, Self::Error> {
        let (tag, rest) = bytes.split_first().ok_or(FrameError::Empty)?;
        match *tag {
            OCI => Request::decode(rest).map(Frame::Oci).map_err(FrameError::Oci),
            POSTGRES => Ok(Frame::Postgres(rest)),
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
    /// A tag that is neither of this frame's two.
    UnknownTag(u8),
    /// The registry request did not parse.
    Oci(serde_json::Error),
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
        }
    }
}

impl Error for FrameError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            FrameError::Oci(error) => Some(error),
            FrameError::Empty | FrameError::UnknownTag(_) => None,
        }
    }
}
