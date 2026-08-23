//! What a client's response frame carries on a command channel.

use std::error;
use std::fmt;

use crate::decode::Decode;
use crate::encode::{Encode, Writer};
use crate::shared::error::Error;

/// One item the command produced, or the news that it will not produce
/// another.
///
/// The payload of a
/// [`ClientFrame::ChannelResponse`](crate::frame::client::ClientFrame::ChannelResponse)
/// on a channel opened by
/// [`channel_request::Frame::Command`](crate::endpoints::mcp_plugin::run::server::channel_request::Frame::Command).
///
/// A payload leads with one byte saying which — `0` for
/// [`Item`](Self::Item), `1` for [`Error`](Self::Error) — and the rest
/// is that variant's own bytes.
///
/// # One frame per item
///
/// Not one frame per command. A command that yields a thousand rows
/// sends a thousand of these and then finishes the channel, so the
/// plugin sees each as it lands rather than waiting for a document
/// assembled from all of them.
///
/// Which makes the channel's own finish the end of the command, and
/// there is no terminator in the payload because a second signal for
/// one fact is a second thing to disagree about.
///
/// # An error is not an item
///
/// And the tag is what keeps them apart. Before it, a command that
/// failed had to say so IN an item, in whatever shape the CLI used for
/// that — which meant a plugin could only find out by inspecting
/// something it was otherwise supposed to pass along, and a plugin that
/// did not know the shape could not find out at all.
///
/// [`Error`](Self::Error) is the caller saying the command did not
/// finish. What arrived before it is what the command produced;
/// nothing follows it, because the channel finishes after.
///
/// # The item is opaque, and the tag does not change that
///
/// What a command produces belongs to the CLI, which gains subcommands
/// on its own schedule. The tag says whether there is an item, not what
/// is in one — so a provider still relays and never reads, in this
/// direction as in the other.
#[derive(Debug, Clone, PartialEq)]
pub enum Frame<'a> {
    /// One item, borrowed from the frame it arrived in. Tag `0`.
    Item(&'a [u8]),
    /// The command did not finish. Tag `1`.
    ///
    /// See [`shared::error::Error`](crate::shared::error::Error) for
    /// why it says so little.
    Error(Error),
}

/// Tag for [`Frame::Item`].
const ITEM: u8 = 0;

/// Tag for [`Frame::Error`].
const ERROR: u8 = 1;

impl Encode for Frame<'_> {
    /// The ordinary JSON failure, from the only variant that has one.
    /// An item is bytes copied.
    type Error = serde_json::Error;

    // Spelled out rather than `Self::Error`: this enum has a variant
    // called `Error`, so the associated type is ambiguous by that name.
    fn encode(&self, out: &mut Writer<'_>) -> Result<(), serde_json::Error> {
        match self {
            Frame::Item(item) => {
                out.extend_from_slice(&[ITEM]);
                out.extend_from_slice(item);
                Ok(())
            }
            Frame::Error(error) => {
                out.extend_from_slice(&[ERROR]);
                error.encode(out)
            }
        }
    }
}

impl<'a> Decode<'a> for Frame<'a> {
    /// Three ways to fail, and only one of them is JSON.
    type Error = FrameError;

    // Spelled out for the same reason as `encode` above.
    fn decode(bytes: &'a [u8]) -> Result<Self, FrameError> {
        let (tag, rest) = bytes.split_first().ok_or(FrameError::Empty)?;
        match *tag {
            ITEM => Ok(Frame::Item(rest)),
            ERROR => Error::decode(rest)
                .map(Frame::Error)
                .map_err(FrameError::Error),
            tag => Err(FrameError::UnknownTag(tag)),
        }
    }
}

/// A command response that could not be read.
#[derive(Debug)]
pub enum FrameError {
    /// No bytes at all, so not even a tag.
    ///
    /// Distinct from an empty item, which is a tag byte followed by
    /// nothing and is a command that produced something with no bytes
    /// in it.
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
                f.write_str("command response frame is empty")
            }
            FrameError::UnknownTag(tag) => {
                write!(f, "unknown command response frame tag {tag}")
            }
            FrameError::Error(error) => {
                write!(f, "command error did not parse: {error}")
            }
        }
    }
}

impl error::Error for FrameError {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match self {
            FrameError::Error(error) => Some(error),
            FrameError::Empty | FrameError::UnknownTag(_) => None,
        }
    }
}
