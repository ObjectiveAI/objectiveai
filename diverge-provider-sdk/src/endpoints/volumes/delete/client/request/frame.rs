//! What a client's request frame carries for a volume deletion.

use serde::{Deserialize, Serialize};

use crate::decode::Decode;
use crate::encode::{Encode, Writer};

/// Destroy a volume and everything in it.
///
/// # A name, and the same access model as everything else
///
/// The one field is a
/// [`Volume::name`](crate::endpoints::volumes::list::server::response::Volume::name)
/// from a listing. A caller cannot delete a volume it was not offered,
/// cannot reach one by naming components, and cannot probe for what
/// exists by deleting and reading the error — because a path it
/// invents is not something this request can express.
///
/// Which is the same protection a
/// [`watch`](crate::endpoints::volumes::watch) has, and it matters
/// more here: a watch that resolved a name wrongly shows a caller
/// something, and this one destroys it.
///
/// # It is not undoable and there is no confirmation
///
/// One frame destroys the volume. Nothing here asks twice, because a
/// protocol that asked twice would be asking a program, and a program
/// answers the second time exactly as it answered the first.
/// Confirmation belongs where a person is, which is above this.
///
/// # What it does to whatever is using it
///
/// Nothing here says, because nothing here can. A volume may be
/// [`VolumeMount`](crate::shared::containers::request::VolumeMount)ed
/// into a running laboratory, or being
/// [`watch`](crate::endpoints::volumes::watch)ed, or both, by this
/// caller and by nobody else — and what a provider does about that is
/// the provider's: refuse while it is in use, or delete it and let the
/// mount fail the way a yanked disk fails.
///
/// A caller that cares stops using it first. That is not a courtesy
/// this protocol can enforce, and pretending otherwise would be
/// promising a coordination it has no frame for.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub struct Frame {
    /// Which volume, by the name a listing gave it.
    ///
    /// Names come from
    /// [`Volume::name`](crate::endpoints::volumes::list::server::response::Volume::name)
    /// and mean nothing outside the provider that published them.
    pub name: String,
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
const TAG: u8 = 8;

/// Postcard, matching the rest of [`volumes`](crate::endpoints::volumes).
impl Encode for Frame {
    /// Postcard's own failure. The tag cannot fail.
    type Error = postcard::Error;

    fn encode(&self, out: &mut Writer<'_>) -> Result<(), Self::Error> {
        out.extend_from_slice(&[TAG]);
        postcard::to_io(self, &mut *out)?;
        Ok(())
    }
}

impl Decode<'_> for Frame {
    /// Three ways to fail, and only one of them is postcard's.
    type Error = FrameError;

    fn decode(bytes: &[u8]) -> Result<Self, Self::Error> {
        let (tag, rest) = bytes.split_first().ok_or(FrameError::Empty)?;
        if *tag != TAG {
            return Err(FrameError::UnexpectedTag(*tag));
        }
        postcard::from_bytes(rest).map_err(FrameError::Body)
    }
}

/// A volume deletion request that could not be read.
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
    Body(postcard::Error),
}

impl std::fmt::Display for FrameError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FrameError::Empty => {
                f.write_str("volume deletion request frame is empty")
            }
            FrameError::UnexpectedTag(tag) => {
                write!(
                    f,
                    "expected volume deletion request tag {TAG}, found {tag}"
                )
            }
            FrameError::Body(error) => {
                write!(f, "volume deletion request did not parse: {error}")
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
