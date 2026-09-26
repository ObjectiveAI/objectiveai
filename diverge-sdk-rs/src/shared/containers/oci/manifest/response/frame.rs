//! The manifest, whole.

use super::{EncodeError, FrameError};
use crate::wire::decode::Decode;
use crate::wire::encode::{Encode, Writer};

/// A manifest's media type and its bytes, in one frame.
///
/// The media type rides along because the runtime needs it back as
/// `Content-Type` and it is not recoverable from the bytes: an image
/// manifest and an image index are both JSON objects, and the
/// `mediaType` field inside them is optional. The bytes are the
/// manifest verbatim — the digest is of these bytes, so nothing may
/// re-serialize them.
///
/// # Two parts on the wire
///
/// `[u16 BE: byte length of the media type][media type][everything
/// after: the manifest bytes]`. The media type leads so the bytes can
/// be the rest, and it is short enough that two bytes of length are
/// plenty.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Frame<'a> {
    /// The manifest's media type —
    /// `application/vnd.oci.image.manifest.v1+json`,
    /// `application/vnd.oci.image.index.v1+json`, or the Docker
    /// equivalents.
    pub media_type: &'a str,
    /// The manifest's bytes, borrowed from the frame they arrived in.
    pub body: &'a [u8],
}

/// The bytes the media type's length occupies.
const MEDIA_TYPE_LEN: usize = 2;

impl Encode for Frame<'_> {
    /// One way to fail: a media type longer than two bytes can say.
    type Error = EncodeError;

    fn encode(&self, out: &mut Writer<'_>) -> Result<(), EncodeError> {
        let media_type = self.media_type.as_bytes();
        let len = u16::try_from(media_type.len())
            .map_err(|_| EncodeError::MediaTypeLength(media_type.len()))?;
        out.extend_from_slice(&len.to_be_bytes());
        out.extend_from_slice(media_type);
        out.extend_from_slice(self.body);
        Ok(())
    }
}

impl<'a> Decode<'a> for Frame<'a> {
    /// Three ways to fail, none of them a parse of the manifest.
    type Error = FrameError;

    fn decode(bytes: &'a [u8]) -> Result<Self, FrameError> {
        if bytes.len() < MEDIA_TYPE_LEN {
            return Err(FrameError::Short(bytes.len()));
        }
        let (len, rest) = bytes.split_at(MEDIA_TYPE_LEN);
        let len = usize::from(u16::from_be_bytes(
            <[u8; MEDIA_TYPE_LEN]>::try_from(len).expect("split_at gave 2 bytes"),
        ));
        if rest.len() < len {
            return Err(FrameError::Truncated {
                need: len,
                have: rest.len(),
            });
        }
        let (media_type, body) = rest.split_at(len);
        Ok(Frame {
            media_type: std::str::from_utf8(media_type)
                .map_err(|_| FrameError::MediaTypeUtf8)?,
            body,
        })
    }
}
