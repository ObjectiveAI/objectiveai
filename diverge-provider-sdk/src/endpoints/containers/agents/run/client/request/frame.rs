//! What a client's request frame carries for an agent container run.

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::decode::Decode;
use crate::encode::{Encode, Writer};
use crate::shared::containers::request::Container;

/// Ask a provider to create an agent container, and say what its loop
/// runs.
///
/// A [`Container`] — the image, the limits, the mounts — and then the
/// two things a loop needs: a prompt, and an agent. They are on the
/// request rather than on the channel that starts the loop because a
/// container runs one loop, on one prompt, as one agent, for its
/// whole life; the channel that starts it has nothing left to say.
///
/// The two are typed to different depths on purpose. The prompt is a
/// string, because every loop takes one and this crate can say so.
/// The agent is a JSON value, because this crate does not know what
/// an agent is — a model, a set of tools, a personality, a harness's
/// own knobs — and a wire that typed it would have to be revised for
/// every agent that ever ran. What the value MAY be is what
/// [`agent_schema`](crate::shared::containers::agent_schema) answers.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Frame {
    /// The container: image, limits, mounts. Flattened, so the wire
    /// is one object rather than a container inside a request.
    #[serde(flatten)]
    pub container: Container,
    /// What the loop is asked. POST-TRANSFORM: the result of whatever
    /// built the request — a system prompt applied, a history folded
    /// in — so a provider never rewrites what it was given.
    pub prompt: String,
    /// The agent, as the image defines it. The typed agents this
    /// crate once carried are kept in
    /// [`agent`](crate::endpoints::containers::agents::agent) for
    /// reference; nothing here reads them.
    pub agent: Value,
}

/// This frame's tag among the scope-opening requests.
///
/// One byte at the front of the payload, which is what tells a reader
/// which request it holds. The frame layer does not discriminate them
/// — [`ClientFrame::Request`](crate::frame::client::ClientFrame::Request)
/// is one type carrying bytes — so the distinction has to be in the
/// bytes, and each request owns the value that names it.
///
/// See the table in [`endpoints`](crate::endpoints) for the whole
/// allocation. The values are chosen across modules that do not know
/// about each other, so the table is the only place they can be seen
/// at once.
const TAG: u8 = 0;

/// JSON, matching [`images::check`](crate::endpoints::images::check)
/// rather than the postcard [`volumes`](crate::endpoints::volumes)
/// uses. One of these is sent per container rather than per
/// filesystem event, so there is no throughput to optimize for — and
/// it names an image the same way a check does, which is reason
/// enough for the two to look alike on the wire. The agent being a
/// [`Value`] settles it besides: a value cannot come back out of
/// postcard at all.
impl Encode for Frame {
    /// The ordinary JSON failure. The tag cannot fail.
    type Error = serde_json::Error;

    fn encode(&self, out: &mut Writer<'_>) -> Result<(), Self::Error> {
        out.extend_from_slice(&[TAG]);
        serde_json::to_writer(out, self)
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
        serde_json::from_slice(rest).map_err(FrameError::Body)
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
