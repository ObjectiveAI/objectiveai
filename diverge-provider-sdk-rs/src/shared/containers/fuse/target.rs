//! What an ask names: a mount, and an entry in it.

use super::{RequestEncodeError, RequestError, prefixed};
use crate::encode::{Encode, Writer};

/// One entry of one mount: the mount's id, and the entry's path
/// relative to the mount root — empty for a file mount, and for a
/// directory mount's root.
///
/// ```text
/// [id_len: u16 BE][id: utf8…][path: utf8…]
/// ```
///
/// The path runs to the end of the payload, so it needs no prefix and
/// takes any length. This is the whole ask for [`read`](super::read),
/// [`list`](super::list), [`remove`](super::remove) and
/// [`mkdir`](super::mkdir), each of which is answered by its own
/// frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Target<'a> {
    /// The mount's id.
    pub id: &'a str,
    /// The entry's path inside the mount, `/`-separated, no leading
    /// slash; empty is the mount itself.
    pub path: &'a str,
}

impl Encode for Target<'_> {
    /// One way to fail: an id longer than the length prefix holds.
    type Error = RequestEncodeError;

    fn encode(&self, out: &mut Writer<'_>) -> Result<(), RequestEncodeError> {
        prefixed::put(out, self.id.as_bytes()).map_err(RequestEncodeError::IdLength)?;
        out.extend_from_slice(self.path.as_bytes());
        Ok(())
    }
}

impl<'a> Target<'a> {
    /// Decode from the bytes after the ask's kind. The id and the path
    /// borrow from `bytes`.
    pub fn decode(bytes: &'a [u8]) -> Result<Self, RequestError> {
        let (id, path) = prefixed::take(bytes)?;
        Ok(Target {
            id: std::str::from_utf8(id).map_err(|_| RequestError::IdUtf8)?,
            path: std::str::from_utf8(path).map_err(|_| RequestError::PathUtf8)?,
        })
    }
}
