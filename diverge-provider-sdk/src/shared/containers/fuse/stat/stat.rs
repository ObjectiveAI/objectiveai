//! What an entry is and how long it is.

use super::super::{Kind, ResponseError};
use crate::encode::Writer;

/// An entry's kind and size, as a [`stat`](super) answers them.
///
/// ```text
/// [kind: u8][size: u64 BE]
/// ```
///
/// The size is the file's byte length, and `0` for a directory. Nine
/// bytes, fixed: everything a `stat(2)` on the mount needs that the
/// proxy does not answer itself.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Stat {
    /// File or directory.
    pub kind: Kind,
    /// The file's length in bytes; `0` for a directory.
    pub size: u64,
}

/// The bytes a stat occupies.
const FIXED: usize = 1 + 8;

impl Stat {
    /// Write the nine bytes.
    pub(crate) fn encode(&self, out: &mut Writer<'_>) {
        out.extend_from_slice(&[self.kind.byte()]);
        out.extend_from_slice(&self.size.to_be_bytes());
    }

    /// Read the nine bytes off the front: the stat, then the rest.
    pub(crate) fn decode(bytes: &[u8]) -> Result<(Self, &[u8]), ResponseError> {
        let fixed = bytes.get(..FIXED).ok_or(ResponseError::Truncated)?;
        let kind = Kind::from_byte(fixed[0])?;
        let size: [u8; 8] = fixed[1..].try_into().expect("eight bytes were taken");
        Ok((
            Stat {
                kind,
                size: u64::from_be_bytes(size),
            },
            &bytes[FIXED..],
        ))
    }
}
