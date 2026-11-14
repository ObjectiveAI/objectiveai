//! What a client's request frame carries for a read scope.

use crate::shared::containers::oci;
use crate::wire::decode::Decode;
use crate::wire::encode::{Encode, Writer};

/// Open a read scope on a provider: the question whether it can
/// supply the ONE image this request names — the
/// [`Image`](crate::shared::containers::request::Image), its digest
/// and the references under which its bytes may be fetched, as the
/// shared [`oci::has`](crate::shared::containers::oci::has) request
/// carries it — and, where it can, a scope to ask on channels for
/// that image's manifests, blobs and proof.
///
/// The digest is the image's identity and a reference is only a
/// locator — any registry serving these bytes serves the same image,
/// because a client recomputes the hash on pull and rejects a
/// mismatch. The references are where the client knows the bytes to
/// be, and a client that knows of none sends none; a provider asks a
/// referenced registry only where it is configured with it. Where it
/// looks, and in what order, is stated on [`read`](super::super::super).
///
/// # On the wire
///
/// The tag, then the shared request's JSON, which is exactly the
/// image's: `{"digest":…,"references":[…]}`, `references` absent when
/// there are none.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Frame(
    /// The image asked about, and served where held.
    pub oci::has::request::Request,
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
const TAG: u8 = 20;

impl Encode for Frame {
    /// The ordinary JSON failure. The tag cannot fail.
    type Error = serde_json::Error;

    fn encode(&self, out: &mut Writer<'_>) -> Result<(), Self::Error> {
        out.extend_from_slice(&[TAG]);
        self.0.encode(out)
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
        oci::has::request::Request::decode(rest).map(Frame).map_err(FrameError::Body)
    }
}

/// A read scope's request that could not be read.
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
            FrameError::Empty => f.write_str("image read request frame is empty"),
            FrameError::UnexpectedTag(tag) => write!(f, "expected image read request tag {TAG}, found {tag}"),
            FrameError::Body(error) => write!(f, "image read request did not parse: {error}"),
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
