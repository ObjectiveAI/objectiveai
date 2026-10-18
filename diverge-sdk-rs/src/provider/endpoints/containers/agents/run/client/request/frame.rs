//! What a client's request frame carries for an agent container run.

use crate::wire::decode::Decode;
use crate::wire::encode::{Encode, Writer};
use crate::shared::containers::request::Container;

/// Ask a provider to create an agent container.
///
/// A [`Container`] and nothing else: the image, the limits, the
/// mounts, the arguments. What makes it an agent container rather
/// than the other kind is not in the request — it is the image, and
/// it is what the caller does on the channels once it runs: a message
/// for the agent, and its conversation on the scope's own main stream.
#[derive(Debug, Clone, PartialEq)]
pub struct Frame(
    /// What to run.
    pub Container,
);

/// This frame's tag among the scope-opening requests.
///
/// One byte at the front of the payload, which is what tells a reader
/// which request it holds. The frame layer does not discriminate them
/// — [`ClientFrame::Request`](crate::wire::frame::client::ClientFrame::Request)
/// is one type carrying bytes — so the distinction has to be in the
/// bytes, and each request owns the value that names it.
///
/// See the table in [`endpoints`](crate::provider::endpoints) for the whole
/// allocation. The values are chosen across modules that do not know
/// about each other, so the table is the only place they can be seen
/// at once.
const TAG: u8 = 0;

/// JSON, matching [`images::check`](crate::provider::endpoints::images::check)
/// rather than the postcard [`volumes`](crate::provider::endpoints::volumes)
/// uses. One of these is sent per container rather than per
/// filesystem event, so there is no throughput to optimize for — and
/// it names an image the same way a check does, which is reason
/// enough for the two to look alike on the wire. The arguments being
/// a [`Value`](serde_json::Value) settles it besides: a value cannot
/// come back out of postcard at all.
impl Encode for Frame {
    /// The ordinary JSON failure. The tag cannot fail.
    type Error = serde_json::Error;

    fn encode(&self, out: &mut Writer<'_>) -> Result<(), Self::Error> {
        out.extend_from_slice(&[TAG]);
        serde_json::to_writer(out, &self.0)
    }
}

impl Decode<'_> for Frame {
    /// Three ways to fail, and only one of them is JSON.
    type Error = FrameError;

    fn decode(bytes: &[u8]) -> Result<Self, Self::Error> {
        let (tag, rest) = bytes.split_first().ok_or(FrameError::Empty)?;
        if *tag != TAG {
            return Err(FrameError::UnexpectedTag(*tag));
        }
        serde_json::from_slice(rest).map(Frame).map_err(FrameError::Body)
    }
}

/// An agent container run request that could not be read.
#[derive(Debug)]
pub enum FrameError {
    /// No bytes at all, so not even a tag.
    Empty,
    /// A tag naming some other request.
    ///
    /// A reader that dispatched on the tag will not see this. One that
    /// assumed which request it held, and was wrong, will — which is
    /// the point of checking a tag rather than skipping it.
    UnexpectedTag(u8),
    /// The request did not parse.
    Body(serde_json::Error),
}

impl std::fmt::Display for FrameError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FrameError::Empty => {
                f.write_str("agents run request frame is empty")
            }
            FrameError::UnexpectedTag(tag) => {
                write!(f, "expected agents run request tag {TAG}, found {tag}")
            }
            FrameError::Body(error) => {
                write!(f, "agents run request did not parse: {error}")
            }
        }
    }
}

impl std::error::Error for FrameError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            FrameError::Body(error) => Some(error),
            FrameError::Empty | FrameError::UnexpectedTag(_) => None,
        }
    }
}
