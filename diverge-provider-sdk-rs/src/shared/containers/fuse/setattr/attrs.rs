//! Which attributes to set, and to what.

use super::super::{RequestError, Time, time::TIME_LEN};
use crate::encode::Writer;

/// The attributes a setattr may change, each set or left: the mode,
/// the owner, the group, the access time and the modification time.
/// The change time is not here — it is the caller's to stamp when
/// anything else changes.
///
/// ```text
/// [set: u8][mode: u32 BE][uid: u32 BE][gid: u32 BE][atime][mtime]
/// ```
///
/// Twenty-nine bytes, fixed: a flags byte saying which fields are
/// set — bit `0` the mode, `1` the owner, `2` the group, `3` the
/// access time, `4` the modification time — and every field's bytes
/// whether set or not, zero when not. Fixed rather than optional so
/// a path may follow and run to the end.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Attrs {
    /// The permission bits, `0o644` and the like.
    pub mode: Option<u32>,
    /// The owner's id.
    pub uid: Option<u32>,
    /// The group's id.
    pub gid: Option<u32>,
    /// When the entry was last read.
    pub atime: Option<Time>,
    /// When the entry's content last changed.
    pub mtime: Option<Time>,
}

/// The bytes the attributes occupy.
pub(crate) const ATTRS_LEN: usize = 1 + 4 + 4 + 4 + 2 * TIME_LEN;

const MODE: u8 = 1 << 0;
const UID: u8 = 1 << 1;
const GID: u8 = 1 << 2;
const ATIME: u8 = 1 << 3;
const MTIME: u8 = 1 << 4;

impl Attrs {
    /// Whether anything is set.
    pub fn is_empty(&self) -> bool {
        self.mode.is_none() && self.uid.is_none() && self.gid.is_none() && self.atime.is_none() && self.mtime.is_none()
    }

    /// Write the twenty-nine bytes.
    pub(crate) fn encode(&self, out: &mut Writer<'_>) {
        let mut set = 0u8;
        set |= self.mode.map_or(0, |_| MODE);
        set |= self.uid.map_or(0, |_| UID);
        set |= self.gid.map_or(0, |_| GID);
        set |= self.atime.map_or(0, |_| ATIME);
        set |= self.mtime.map_or(0, |_| MTIME);
        out.extend_from_slice(&[set]);
        out.extend_from_slice(&self.mode.unwrap_or(0).to_be_bytes());
        out.extend_from_slice(&self.uid.unwrap_or(0).to_be_bytes());
        out.extend_from_slice(&self.gid.unwrap_or(0).to_be_bytes());
        self.atime.unwrap_or_default().encode(out);
        self.mtime.unwrap_or_default().encode(out);
    }

    /// Read the twenty-nine bytes off the front: the attributes, then
    /// the rest.
    pub(crate) fn decode(bytes: &[u8]) -> Result<(Self, &[u8]), RequestError> {
        if bytes.len() < ATTRS_LEN {
            return Err(RequestError::Truncated);
        }
        let set = bytes[0];
        let mode: [u8; 4] = bytes[1..5].try_into().expect("four bytes were taken");
        let uid: [u8; 4] = bytes[5..9].try_into().expect("four bytes were taken");
        let gid: [u8; 4] = bytes[9..13].try_into().expect("four bytes were taken");
        let (atime, rest) = Time::decode(&bytes[13..]).map_err(|_| RequestError::Truncated)?;
        let (mtime, rest) = Time::decode(rest).map_err(|_| RequestError::Truncated)?;
        Ok((
            Attrs {
                mode: (set & MODE != 0).then_some(u32::from_be_bytes(mode)),
                uid: (set & UID != 0).then_some(u32::from_be_bytes(uid)),
                gid: (set & GID != 0).then_some(u32::from_be_bytes(gid)),
                atime: (set & ATIME != 0).then_some(atime),
                mtime: (set & MTIME != 0).then_some(mtime),
            },
            rest,
        ))
    }
}
