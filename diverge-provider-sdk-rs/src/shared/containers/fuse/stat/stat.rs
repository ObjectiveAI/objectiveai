//! What an entry is: its kind, its size, and the six the kernel asks
//! for beside them.

use super::super::{Kind, ResponseError, Time, time::TIME_LEN};
use crate::encode::Writer;

/// An entry's attributes, as a [`stat`](super) answers them.
///
/// ```text
/// [kind: u8][size: u64 BE][mode: u32 BE][uid: u32 BE][gid: u32 BE][atime][mtime][ctime]
/// ```
///
/// Fifty-seven bytes, fixed: everything a `stat(2)` on the mount
/// needs that the proxy does not answer itself. The size is the
/// file's byte length, and `0` for a directory. The mode is the
/// permission bits alone, `0o644` and the like, never the kind — the
/// kind is its own byte. The owner and the group are ids as the
/// caller's storage holds them, and the three times are
/// [`Time`]s: last access, last modification, last change of the
/// attributes themselves.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Stat {
    /// File or directory.
    pub kind: Kind,
    /// The file's length in bytes; `0` for a directory.
    pub size: u64,
    /// The permission bits, `0o644` and the like.
    pub mode: u32,
    /// The owner's id.
    pub uid: u32,
    /// The group's id.
    pub gid: u32,
    /// When the entry was last read.
    pub atime: Time,
    /// When the entry's content last changed.
    pub mtime: Time,
    /// When the entry's attributes last changed.
    pub ctime: Time,
}

/// The bytes a stat occupies.
const FIXED: usize = 1 + 8 + 4 + 4 + 4 + 3 * TIME_LEN;

impl Stat {
    /// Write the fifty-seven bytes.
    pub(crate) fn encode(&self, out: &mut Writer<'_>) {
        out.extend_from_slice(&[self.kind.byte()]);
        out.extend_from_slice(&self.size.to_be_bytes());
        out.extend_from_slice(&self.mode.to_be_bytes());
        out.extend_from_slice(&self.uid.to_be_bytes());
        out.extend_from_slice(&self.gid.to_be_bytes());
        self.atime.encode(out);
        self.mtime.encode(out);
        self.ctime.encode(out);
    }

    /// Read the fifty-seven bytes off the front: the stat, then the
    /// rest.
    pub(crate) fn decode(bytes: &[u8]) -> Result<(Self, &[u8]), ResponseError> {
        if bytes.len() < FIXED {
            return Err(ResponseError::Truncated);
        }
        let kind = Kind::from_byte(bytes[0])?;
        let size: [u8; 8] = bytes[1..9].try_into().expect("eight bytes were taken");
        let mode: [u8; 4] = bytes[9..13].try_into().expect("four bytes were taken");
        let uid: [u8; 4] = bytes[13..17].try_into().expect("four bytes were taken");
        let gid: [u8; 4] = bytes[17..21].try_into().expect("four bytes were taken");
        let (atime, rest) = Time::decode(&bytes[21..])?;
        let (mtime, rest) = Time::decode(rest)?;
        let (ctime, rest) = Time::decode(rest)?;
        Ok((
            Stat {
                kind,
                size: u64::from_be_bytes(size),
                mode: u32::from_be_bytes(mode),
                uid: u32::from_be_bytes(uid),
                gid: u32::from_be_bytes(gid),
                atime,
                mtime,
                ctime,
            },
            rest,
        ))
    }
}
