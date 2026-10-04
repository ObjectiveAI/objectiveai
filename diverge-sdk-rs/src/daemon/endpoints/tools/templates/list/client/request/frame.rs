//! What a client's request frame carries for a list.

use crate::wire::decode::Decode;
use crate::wire::encode::{Encode, Writer};
use serde::{Deserialize, Serialize};

use super::Filter;

/// Ask the daemon for the caller's templates, narrowed.
///
/// Everything is optional, and a request with none of it — `{}` on the
/// wire — is every template of the caller's. The members but `jq` and
/// `count` together are the filter. The daemon applies the filter
/// first, oldest created first, so the program sees only what it lets
/// through, and runs the program over each template of that; what the
/// program yields is what comes back, and without a program the
/// templates come back as they are. The count caps what comes back.
/// [`Listed`](crate::daemon::endpoints::tools::templates::list::server::response::Listed)
/// is the shape each comes back in without a program, and the reference
/// for what a program is run over.
///
/// # Any one of, every one of
///
/// A member that lists candidates — `ids`, `creators` — matches a
/// template that is any one of them, or was made by any one of them.
/// `all_tags` matches a template that carries every one of them, and
/// `any_tags` one that carries any one of them. An empty list is
/// absent, and matches every template.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct Frame {
    /// The filter: see [`Filter`]. Flattened, so its members are
    /// this object's own.
    #[serde(flatten)]
    pub filter: Filter,
    /// How many values to send at most, counting what comes back —
    /// templates as they are, or what the program yields — and not what
    /// the filter reads; once that many have been sent the scope
    /// finishes, whether or not more would have matched. `0` sends
    /// nothing and finishes at once. Absent, no cap.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub count: Option<u64>,
}

/// This frame's tag among the scope-opening requests.
///
/// One byte at the front of the payload, which is what tells a reader
/// which request it holds. The frame layer does not discriminate them —
/// [`ClientFrame::Request`](crate::wire::frame::client::ClientFrame::Request)
/// is one type carrying bytes — so the distinction has to be in the
/// bytes, and each request owns the value that names it.
///
/// See the table in [`endpoints`](crate::daemon::endpoints) for the
/// whole allocation. The values are chosen across modules that do not
/// know about each other, so the table is the only place they can be
/// seen at once.
const TAG: u8 = 30;

/// JSON, as every request of the daemon's is: the filter's members,
/// each absent when it says nothing, so that a request that says
/// nothing is `{}`.
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

/// A tools templates list request frame that could not be read.
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
            FrameError::Empty => f.write_str("tools templates list request frame is empty"),
            FrameError::UnexpectedTag(tag) => {
                write!(f, "expected tools templates list request tag {TAG}, found {tag}")
            }
            FrameError::Body(error) => {
                write!(f, "tools templates list request did not parse: {error}")
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
