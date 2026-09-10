//! One entry of a listed directory.

use super::{ResponseEncodeError, ResponseError, prefixed};
use crate::encode::Writer;

/// What an entry is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Kind {
    /// A regular file. Kind `0`.
    File,
    /// A directory. Kind `1`.
    Directory,
}

/// One entry of a directory, as a [`list`](super::list) answers it.
///
/// ```text
/// [kind: u8][size: u64 BE][name_len: u16 BE][name: utf8…]
/// ```
///
/// The size is the file's byte length, and `0` for a directory. The
/// name is the entry's own, one path component, never `.` or `..`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Entry<'a> {
    /// The entry's name.
    pub name: &'a str,
    /// File or directory.
    pub kind: Kind,
    /// The file's length in bytes; `0` for a directory.
    pub size: u64,
}

/// Kind byte for a file.
const FILE: u8 = 0;

/// Kind byte for a directory.
const DIRECTORY: u8 = 1;

/// The bytes the fixed part of an entry occupies: the kind and the
/// size.
const FIXED: usize = 1 + 8;

impl Entry<'_> {
    /// Write one entry.
    pub(crate) fn encode(&self, out: &mut Writer<'_>) -> Result<(), ResponseEncodeError> {
        out.extend_from_slice(&[match self.kind {
            Kind::File => FILE,
            Kind::Directory => DIRECTORY,
        }]);
        out.extend_from_slice(&self.size.to_be_bytes());
        prefixed::put(out, self.name.as_bytes()).map_err(ResponseEncodeError::NameLength)
    }
}

impl<'a> Entry<'a> {
    /// Split one entry off the front: the entry, then the rest.
    pub(crate) fn decode(bytes: &'a [u8]) -> Result<(Self, &'a [u8]), ResponseError> {
        let fixed = bytes.get(..FIXED).ok_or(ResponseError::Truncated)?;
        let kind = match fixed[0] {
            FILE => Kind::File,
            DIRECTORY => Kind::Directory,
            other => return Err(ResponseError::UnknownKind(other)),
        };
        let size: [u8; 8] = fixed[1..].try_into().expect("eight bytes were taken");
        let (name, rest) =
            prefixed::take(&bytes[FIXED..]).map_err(|_| ResponseError::Truncated)?;
        Ok((
            Entry {
                name: std::str::from_utf8(name).map_err(|_| ResponseError::NameUtf8)?,
                kind,
                size: u64::from_be_bytes(size),
            },
            rest,
        ))
    }
}
