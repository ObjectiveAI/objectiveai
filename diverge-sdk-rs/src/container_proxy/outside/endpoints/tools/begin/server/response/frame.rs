//! What a server's response frame carries for a tool container begin.

use std::fmt;

use super::Registered;
use crate::wire::decode::Decode;
use crate::wire::encode::{Encode, Writer};
use crate::shared::containers::secrets::Values;
use crate::shared::error::Error;

/// A begin's answer: that the connection has begun, then the proxy's
/// word whenever the container goes active and inactive, for as long
/// as the connection lives — or a failure.
///
/// A payload leads with one byte saying which — `0` for
/// [`Begun`](Self::Begun), `1` for [`Error`](Self::Error), `2` for
/// [`Active`](Self::Active), `3` for [`Inactive`](Self::Inactive) —
/// and the rest is that variant's own JSON, or nothing: the one bare
/// word carries nothing after the byte.
///
/// # Begun, then the activity
///
/// | the scope | means |
/// |-----------|-------|
/// | a begun, then quiet, and stays open | the connection has begun, what its program declared is known, and no exchange has been in flight |
/// | a begun, an active, and stays open | one or more MCP exchanges are in flight: the container is serving |
/// | …an active, an inactive, and stays open | the last in flight has been answered, its rotating secrets collected; the container is idle until the next |
/// | an error, then a finish | it has not begun — this connection had already begun, or the arguments were refused; or, after `Begun`, the secrets could not be collected |
/// | a finish, with no error | the proxy is ending |
///
/// # The proxy marks the activity, and only the proxy can
///
/// An exchange is one of the four MCP calls the server opens into
/// the container — `tools/list`, `resources/list`, `tools/call`,
/// `resources/read`; notifications are no exchange. The proxy counts
/// them in flight: the count going from none to one is
/// [`Active`](Self::Active), sent before that exchange is delivered,
/// and the count going from one to none is
/// [`Inactive`](Self::Inactive), sent after the last is answered and
/// the program has handed back its rotating secrets, which ride it.
/// Nothing the program says is read as either word.
///
/// # Nothing is delivered before a feed
///
/// After `Begun`, and after every `Inactive`, the proxy delivers
/// nothing to a program that declared a secret until the server has
/// fed the values on a
/// [`Secrets`](super::super::super::client::channel_request::Frame::Secrets)
/// channel and the proxy has answered it `Fed`: an exchange opened
/// meanwhile is held, its `Active` already sent. A program that
/// declared none is never gated.
///
/// Everything else the server reads from the container — the
/// family's own exchange, the asks the container makes — is a
/// channel, not this stream. It carries no readiness signal beyond
/// `Begun`: the proxy is here, the arguments are held, and channels
/// may be opened.
#[derive(Debug, Clone, PartialEq)]
pub enum Frame {
    /// The connection has begun, the container holds its arguments
    /// for its life, and this is what its program declared. Tag `0`.
    ///
    /// Arrives once, when the container's server has taken the
    /// arguments and answered its registration — as JSON, a
    /// [`Registered`]: the secrets it takes, a map, empty for a
    /// program that takes none; a tool container declares no
    /// dependencies, and a list its program answers with is ignored.
    /// See [`registration`](crate::shared::containers::registration)
    /// for what the provider does with it. A channel on this scope is
    /// opened only after it.
    Begun(Registered),
    /// A failure. Tag `1`.
    ///
    /// A second begin on a connection that had one, or arguments the
    /// container refused — the container's server's own words: a
    /// value the image will not take — or, after `Begun`, rotating
    /// secrets the program would not hand back, named and never
    /// valued. It is the one variant that ends the scope rather than
    /// adding to it. See
    /// [`shared::error::Error`](crate::shared::error::Error) for why
    /// it says so little.
    Error(Error),
    /// An exchange is in flight where none was. Tag `2`, and nothing
    /// after it.
    ///
    /// Sent by the proxy before the exchange is delivered, never
    /// relayed from the program; never twice without an
    /// [`Inactive`](Self::Inactive) between.
    Active,
    /// The last exchange in flight has been answered, and these are
    /// the rotating secrets. Tag `3`.
    ///
    /// The program handed back every `rotating` secret it declared,
    /// each by name to the value it leaves in it, as JSON after the
    /// byte: the [`Values`] the caller writes back. Empty for a
    /// program that declared none rotating. Sent by the proxy, never
    /// relayed from the program; never without an
    /// [`Active`](Self::Active) before it.
    Inactive(Values),
}

/// Tag for [`Frame::Begun`].
const BEGUN: u8 = 0;

/// Tag for [`Frame::Error`].
const ERROR: u8 = 1;

/// Tag for [`Frame::Active`]. Public, so the proxy writes the one
/// byte that is the whole frame.
pub const ACTIVE: u8 = 2;

/// Tag for [`Frame::Inactive`]. Public, so a relay that carries the
/// values' JSON without reading it can frame it.
pub const INACTIVE: u8 = 3;

/// A tag, and that variant's own JSON — or the tag alone.
impl Encode for Frame {
    /// The ordinary JSON failure; `Active` cannot fail.
    type Error = serde_json::Error;

    // Spelled out rather than `Self::Error`: this enum has a variant
    // called `Error`, so the associated type is ambiguous by that name.
    fn encode(
        &self,
        out: &mut Writer<'_>,
    ) -> Result<(), serde_json::Error> {
        match self {
            Frame::Begun(registered) => {
                out.extend_from_slice(&[BEGUN]);
                serde_json::to_writer(out, registered)
            }
            Frame::Error(error) => {
                out.extend_from_slice(&[ERROR]);
                error.encode(out)
            }
            Frame::Active => {
                out.extend_from_slice(&[ACTIVE]);
                Ok(())
            }
            Frame::Inactive(values) => {
                out.extend_from_slice(&[INACTIVE]);
                serde_json::to_writer(out, values)
            }
        }
    }
}

impl Decode<'_> for Frame {
    /// Five ways to fail, and three of them are JSON.
    type Error = FrameError;

    // Spelled out for the same reason as `encode` above.
    fn decode(bytes: &[u8]) -> Result<Self, FrameError> {
        let (tag, rest) = bytes.split_first().ok_or(FrameError::Empty)?;
        match *tag {
            BEGUN => serde_json::from_slice(rest).map(Frame::Begun).map_err(FrameError::Begun),
            ERROR => {
                Error::decode(rest).map(Frame::Error).map_err(FrameError::Error)
            }
            ACTIVE => Ok(Frame::Active),
            INACTIVE => serde_json::from_slice(rest)
                .map(Frame::Inactive)
                .map_err(FrameError::Inactive),
            tag => Err(FrameError::UnknownTag(tag)),
        }
    }
}

/// A tools begin response frame that could not be read.
#[derive(Debug)]
pub enum FrameError {
    /// No bytes at all, so not even a tag.
    Empty,
    /// A tag that is none of this frame's four.
    UnknownTag(u8),
    /// What the program declared did not parse.
    Begun(serde_json::Error),
    /// The error did not parse.
    Error(serde_json::Error),
    /// The rotating secrets did not parse. The message is the
    /// parser's, which quotes no value.
    Inactive(serde_json::Error),
}

impl fmt::Display for FrameError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FrameError::Empty => f.write_str("tools begin response frame is empty"),
            FrameError::UnknownTag(tag) => {
                write!(f, "unknown tools begin response frame tag {tag}")
            }
            FrameError::Begun(error) => {
                write!(f, "tools begin registration did not parse: {error}")
            }
            FrameError::Error(error) => {
                write!(f, "tools begin error did not parse: {error}")
            }
            FrameError::Inactive(error) => {
                write!(f, "tools begin inactive secrets did not parse: {error}")
            }
        }
    }
}

impl std::error::Error for FrameError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            FrameError::Begun(error) | FrameError::Error(error) | FrameError::Inactive(error) => Some(error),
            FrameError::Empty | FrameError::UnknownTag(_) => None,
        }
    }
}
