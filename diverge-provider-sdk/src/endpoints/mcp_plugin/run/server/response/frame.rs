//! What a server's response frame carries for an MCP plugin.

use std::fmt;

use crate::decode::Decode;
use crate::encode::{Encode, Writer};
use crate::shared::error::Error;

/// The plugin is running, or it is not.
///
/// One of these on channel `0`. A payload leads with one byte saying
/// which — `0` for [`Ready`](Self::Ready), `1` for
/// [`Error`](Self::Error) — and for the first there is nothing after
/// it, because saying so IS the whole message.
///
/// | the scope ends with | means |
/// |----------------------|-------|
/// | [`Ready`](Self::Ready), then work | the plugin is up; call it |
/// | an [`Error`](Self::Error), then a finish | it never came up, and here is what the provider knows |
///
/// # Why there is no id
///
/// A [`laboratory run`](crate::endpoints::laboratories::run) answers
/// with one, because a laboratory is a place others join: an id is
/// what a [`connect`](crate::endpoints::laboratories::connect) names
/// and what a
/// [`transfer`](crate::shared::container::transfer) sends a file to.
///
/// A plugin is none of those. It is created for one caller, answers
/// that caller's tool calls, and is torn down after — so the scope is
/// the whole of the handle, and an id would name a thing nobody can
/// address. Minting one anyway would be inventing a way to reach a
/// plugin container that this specification deliberately does not
/// offer.
///
/// # Why there is no filetree either
///
/// Because nothing reads it. A laboratory reports its filesystem
/// because an agent works in it and wants to see what it is working
/// on. A plugin serves tools; its filesystem is its author's business,
/// it takes no [`mounts`], and a caller with no way to read or write
/// inside it has nothing to do with a tree of it.
///
/// [`mounts`]: crate::endpoints::laboratories::run::client::request::Frame::mounts
#[derive(Debug, Clone, PartialEq)]
pub enum Frame {
    /// The plugin is up and its MCP server can be reached. Tag `0`.
    Ready,
    /// A failure. Tag `1`.
    ///
    /// The plugin is not running and will not be — the image would not
    /// pull, the container would not start, nothing ever answered on
    /// [`port`](crate::endpoints::mcp_plugin::run::client::request::Frame::port),
    /// whatever the provider knows.
    ///
    /// See [`shared::error::Error`](crate::shared::error::Error) for
    /// why it says so little.
    Error(Error),
}

/// Tag for [`Frame::Ready`].
const READY: u8 = 0;

/// Tag for [`Frame::Error`].
const ERROR: u8 = 1;

impl Encode for Frame {
    /// The ordinary JSON failure, from the half that has one. A lone
    /// tag byte cannot fail.
    type Error = serde_json::Error;

    // Spelled out rather than `Self::Error`: this enum has a variant
    // called `Error`, so the associated type is ambiguous by that name.
    fn encode(
        &self,
        out: &mut Writer<'_>,
    ) -> Result<(), serde_json::Error> {
        match self {
            Frame::Ready => {
                out.extend_from_slice(&[READY]);
                Ok(())
            }
            Frame::Error(error) => {
                out.extend_from_slice(&[ERROR]);
                error.encode(out)
            }
        }
    }
}

impl Decode<'_> for Frame {
    /// Three ways to fail, and only one of them is a parse.
    type Error = FrameError;

    // Spelled out for the same reason as `encode` above.
    fn decode(bytes: &[u8]) -> Result<Self, FrameError> {
        let (tag, rest) = bytes.split_first().ok_or(FrameError::Empty)?;
        match *tag {
            READY => Ok(Frame::Ready),
            ERROR => {
                Error::decode(rest).map(Frame::Error).map_err(FrameError::Error)
            }
            tag => Err(FrameError::UnknownTag(tag)),
        }
    }
}

/// An MCP plugin response frame that could not be read.
#[derive(Debug)]
pub enum FrameError {
    /// No bytes at all, so not even a tag.
    Empty,
    /// A tag that is neither of this frame's two.
    UnknownTag(u8),
    /// The error did not parse.
    Error(serde_json::Error),
}

impl fmt::Display for FrameError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FrameError::Empty => {
                f.write_str("mcp plugin response frame is empty")
            }
            FrameError::UnknownTag(tag) => {
                write!(f, "unknown mcp plugin response frame tag {tag}")
            }
            FrameError::Error(error) => {
                write!(f, "mcp plugin error did not parse: {error}")
            }
        }
    }
}

impl std::error::Error for FrameError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            FrameError::Error(error) => Some(error),
            FrameError::Empty | FrameError::UnknownTag(_) => None,
        }
    }
}
