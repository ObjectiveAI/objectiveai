//! What a client's request frame carries for a connection.

use serde::{Deserialize, Serialize};

use crate::decode::Decode;
use crate::encode::{Encode, Writer};

/// Ask to join a container somebody else is running.
///
/// # What happens to the authorization
///
/// Nothing, here. The provider relays it to whoever holds the
/// container's run scope, as an
/// [`Authorize`](crate::endpoints::laboratories::run::server::channel_request::Frame::Authorize),
/// and the answer to that is whether this scope opens.
///
/// Which is why the credential is opaque. A provider that had to
/// understand it would have to know what makes one connector
/// acceptable and another not, and it does not — the runner does, and
/// the runner is who reads it.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub struct Frame {
    /// The container to join.
    ///
    /// A [`Frame::Id`](crate::endpoints::laboratories::run::server::response::Frame::Id)
    /// from a run. It means nothing to a connector that was not
    /// given it, and nothing outside the provider that minted it.
    pub id: String,
    /// Whatever the runner needs in order to say yes.
    ///
    /// Opaque, and relayed verbatim. A shared secret, a signed token,
    /// a name — this layer does not know and does not look, so nothing
    /// here constrains what a runner chooses to require.
    ///
    /// Text, for the same reason an
    /// [`Auth`](crate::frame::auth::Auth) credential is: what this
    /// carries in practice already is a string, and bytes made a
    /// caller pick an encoding for something that never needed one.
    ///
    /// May be empty, which is a connector offering nothing. Whether
    /// that is ever enough is the runner's to decide.
    pub authorization: String,
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
const TAG: u8 = 3;

/// JSON, like every other structured request.
///
/// It was laid out by hand once — a length-prefixed id and then the
/// authorization running to the end of the payload — because an
/// authorization used to be arbitrary bytes, which JSON can only hold
/// as base64 and which serde would have written as a sequence and read
/// back as a byte string.
///
/// Both are strings now, so none of that applies, and neither does the
/// length: it existed only because two variable-length fields cannot
/// share one end. A format that already delimits its fields does not
/// need to be told where one stops.
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

/// A connection request that could not be read.
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
                f.write_str("connection request frame is empty")
            }
            FrameError::UnexpectedTag(tag) => {
                write!(f, "expected connection request tag {TAG}, found {tag}")
            }
            FrameError::Body(error) => {
                write!(f, "connection request did not parse: {error}")
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
