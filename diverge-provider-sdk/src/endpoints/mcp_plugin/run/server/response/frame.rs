//! What a server's response frame carries for an MCP plugin.

use std::fmt;

use crate::decode::Decode;
use crate::encode::{Encode, Writer};
use crate::shared::error::Error;

/// The plugin did not come up.
///
/// The only thing a run's scope ever says. A payload leads with one
/// byte — `0`, and nothing else is defined — and the rest is the
/// error.
///
/// | the scope | means |
/// |-----------|-------|
/// | says nothing, and stays open | the plugin is running |
/// | one of these, then a finish | it never came up, and here is what the provider knows |
/// | a finish, with none of these | the run is over |
///
/// # Silence is the good case
///
/// Which is unusual here and worth stating plainly: a plugin that
/// works produces no response frame at all. The scope opens, the image
/// is pulled on channels the provider opens, the container starts, and
/// nothing is said about any of it. What a caller does next is call the
/// plugin.
///
/// # Why there is no readiness signal
///
/// There was one, and it was removed, because it could not mean what
/// it appeared to. A provider knows when a CONTAINER has started, and
/// that is not the same fact as the MCP server inside it having bound
/// [`port`](crate::endpoints::mcp_plugin::run::client::request::Frame::port).
/// A signal sent at the first would have been read as the second.
///
/// The honest test is a call. This endpoint already relies on that
/// elsewhere — a wrong `port` is documented as surfacing "as an
/// exchange that finishes without an answer, rather than when the
/// plugin started" — so a caller that wants to know whether the plugin
/// is up asks it something, and a readiness frame would have been a
/// second, weaker answer to a question already answered better.
///
/// It also cost a round of doubt that a caller had no way to resolve:
/// nothing said what to do about a plugin that reported ready and then
/// did not answer.
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
/// # Why a struct with a tag, rather than the error alone
///
/// The tag is what leaves room. One kind of response today is not a
/// promise of one forever, and a payload that was bare error bytes
/// could not grow a second kind without every existing reader
/// misreading it. One byte holds that door open.
///
/// A struct rather than a one-variant enum for the reason this crate
/// uses everywhere: an enum with nothing to choose between implies a
/// decision nobody makes.
///
/// [`mounts`]: crate::endpoints::laboratories::run::client::request::Frame::mounts
#[derive(Debug, Clone, PartialEq)]
pub struct Frame(
    /// What went wrong.
    ///
    /// See [`shared::error::Error`](crate::shared::error::Error) for
    /// why it says so little.
    pub Error,
);

/// Tag for [`Frame`].
const ERROR: u8 = 0;

/// A tag, then the error's JSON.
impl Encode for Frame {
    /// The ordinary JSON failure. The tag cannot fail.
    type Error = serde_json::Error;

    // Spelled out rather than `Self::Error`: this struct's field is a
    // type called `Error`, so the associated type is ambiguous by that
    // name.
    fn encode(&self, out: &mut Writer<'_>) -> Result<(), serde_json::Error> {
        out.extend_from_slice(&[ERROR]);
        self.0.encode(out)
    }
}

impl Decode<'_> for Frame {
    /// Three ways to fail, and only one of them is a parse.
    type Error = FrameError;

    // Spelled out for the same reason as `encode` above.
    fn decode(bytes: &[u8]) -> Result<Self, FrameError> {
        let (tag, rest) = bytes.split_first().ok_or(FrameError::Empty)?;
        match *tag {
            ERROR => Error::decode(rest).map(Frame).map_err(FrameError::Error),
            tag => Err(FrameError::UnknownTag(tag)),
        }
    }
}

/// An MCP plugin response frame that could not be read.
#[derive(Debug)]
pub enum FrameError {
    /// No bytes at all, so not even a tag.
    Empty,
    /// A tag that is not this frame's one.
    ///
    /// Which is how a response kind added later arrives at a reader
    /// built before it — as something unreadable rather than as an
    /// error that was never sent.
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
