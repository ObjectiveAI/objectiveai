//! The byte `0`, or the byte `1` and a name.

use std::convert::Infallible;

use super::FrameError;
use crate::wire::decode::Decode;
use crate::wire::encode::{Encode, Writer};

/// Whether the caller holds the image, and under what repository
/// path: the byte `0` and nothing after it when it does not; the byte
/// `1` followed by the path as UTF-8 when it does.
///
/// The path is the one the caller holds the image under, which the
/// provider's registry serves it from: a runtime pulls
/// `<registry>/<repository>/<name>@<digest>`, and `name` is this. The
/// caller's store is keyed by digest, so the name is the caller's to
/// answer and not the provider's to ask. A provider refuses a path
/// that is not a repository path, since it lands in a reference by
/// concatenation. A name that is empty is not a name: it encodes as
/// not held.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Frame {
    /// The repository path the image is held under, or `None` for not
    /// held.
    pub name: Option<String>,
}

impl Encode for Frame {
    /// A byte and bytes: nothing to fail.
    type Error = Infallible;

    fn encode(&self, out: &mut Writer<'_>) -> Result<(), Infallible> {
        match self.name.as_deref() {
            Some(name) if !name.is_empty() => {
                out.extend_from_slice(&[1]);
                out.extend_from_slice(name.as_bytes());
            }
            _ => out.extend_from_slice(&[0]),
        }
        Ok(())
    }
}

impl Decode<'_> for Frame {
    /// No byte, a first byte that is neither, a `0` with bytes after
    /// it, or a `1` with none after it or with bytes that are not
    /// UTF-8.
    type Error = FrameError;

    fn decode(bytes: &[u8]) -> Result<Self, FrameError> {
        match bytes {
            [] => Err(FrameError::Empty),
            [0] => Ok(Frame { name: None }),
            [0, ..] => Err(FrameError::Trailing),
            [1] => Err(FrameError::Name),
            [1, name @ ..] => std::str::from_utf8(name)
                .map(|name| Frame {
                    name: Some(name.to_string()),
                })
                .map_err(|_| FrameError::Name),
            [byte, ..] => Err(FrameError::Byte(*byte)),
        }
    }
}
