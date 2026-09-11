//! What a client's request frame carries for a volume creation.

use serde::{Deserialize, Serialize};

use crate::decode::Decode;
use crate::encode::{Encode, Writer};

/// Ask a provider for a volume of its own.
///
/// Two fields, and neither says where it goes. A caller names the
/// volume and says how big it is; everything about how a provider
/// satisfies that — a subvolume, a quota, a file with a filesystem in
/// it, a directory on a disk with room to spare — is the provider's,
/// and none of it is expressible here.
///
/// # The name is the caller's, unlike a listed one
///
/// A [`Volume`](crate::endpoints::volumes::list::server::response::Volume)
/// in a listing carries a name the PROVIDER chose, because the
/// provider decided to offer it. This one carries a name the CALLER
/// chose, because the caller asked for it to exist.
///
/// Which is a real difference and the only one. Afterwards the volume
/// is a volume: it appears in a
/// [`list`](crate::endpoints::volumes::list), it can be
/// [`watch`](crate::endpoints::volumes::watch)ed, and a
/// [`VolumeMount`](crate::shared::containers::request::VolumeMount)
/// names it exactly as it names any other. Nothing downstream knows or
/// cares which way it came about.
///
/// # Names have to stay unique
///
/// Because the name is the handle, and a listing carrying two of them
/// makes one unreachable. So a creation that would collide with a
/// volume the caller can already see fails.
///
/// What a provider does about names ACROSS callers is its own affair
/// and invisible from here. Two callers may both create `workspace`
/// and neither will know — a listing shows what that caller can reach,
/// which is the same reason a caller cannot reach a volume by guessing
/// its name.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub struct Frame {
    /// What to call it.
    ///
    /// The handle, from the moment it exists — a
    /// [`watch`](crate::endpoints::volumes::watch) and a
    /// [`VolumeMount`](crate::shared::containers::request::VolumeMount)
    /// name it by this and by nothing else.
    ///
    /// Nothing derives it from anything and nothing constrains it here.
    /// What a provider will accept as a name is the provider's to
    /// state, and a caller learns it by being refused.
    pub name: String,
    /// How big it is, in BYTES.
    ///
    /// The size it starts at. An
    /// [`edit`](crate::endpoints::volumes::edit) changes it later.
    ///
    /// This is what a listing reports back as
    /// [`Volume::bytes`](crate::endpoints::volumes::list::server::response::Volume::bytes),
    /// and what a [`stat`](crate::endpoints::volumes::stat) reports
    /// beside a
    /// [`bytes_used`](crate::endpoints::volumes::stat::server::response::Stat::bytes_used)
    /// that says how much of it is in use.
    ///
    /// Bytes rather than megabytes because a unit that has to be
    /// spelled out in prose is a unit half of everyone gets wrong.
    pub bytes: u64,
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
const TAG: u8 = 7;

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

/// A volume creation request that could not be read.
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
                f.write_str("volume creation request frame is empty")
            }
            FrameError::UnexpectedTag(tag) => {
                write!(
                    f,
                    "expected volume creation request tag {TAG}, found {tag}"
                )
            }
            FrameError::Body(error) => {
                write!(f, "volume creation request did not parse: {error}")
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
