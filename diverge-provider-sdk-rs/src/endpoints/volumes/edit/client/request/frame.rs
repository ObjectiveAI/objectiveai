//! What a client's request frame carries for a volume edit.

use serde::{Deserialize, Serialize};

use super::Change;
use crate::decode::Decode;
use crate::encode::{Encode, Writer};

/// Change a volume's size, its persist mode, or both.
///
/// The two things about a volume that can be edited, and an edit is
/// one [`Change`]: the size, the mode, or both at once. Its
/// [`name`](Self::name) cannot be edited — the name is the handle,
/// and a handle that changed would leave every
/// [`VolumeMount`](crate::shared::containers::request::VolumeMount)
/// naming something that is no longer there.
///
/// So this names the volume and states the change. Two fields, and
/// only one of them is being set.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Frame {
    /// Which volume, by the name a listing gave it.
    ///
    /// Names come from
    /// [`Volume::name`](crate::endpoints::volumes::list::server::response::Volume::name)
    /// and mean nothing outside the provider that published them.
    pub name: String,
    /// What to change: the size, the persist mode, or both. See
    /// [`Change`].
    pub change: Change,
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

/// A volume edit request that could not be read.
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
                f.write_str("volume edit request frame is empty")
            }
            FrameError::UnexpectedTag(tag) => {
                write!(
                    f,
                    "expected volume edit request tag {TAG}, found {tag}"
                )
            }
            FrameError::Body(error) => {
                write!(f, "volume edit request did not parse: {error}")
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
