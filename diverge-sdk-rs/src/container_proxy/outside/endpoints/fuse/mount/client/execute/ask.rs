//! One ask a mount makes, owned.

use bytes::Bytes;

use super::super::super::server::channel_request::Frame;
use crate::shared::containers::fuse::Attrs;

/// What a mount asks the server for, owned, so it outlives the
/// message it arrived in and is answered whenever the caller answers.
/// No mount id: the scope the ask arrived on is the mount, and the
/// server puts the caller's id back when it relays.
#[derive(Debug, Clone)]
pub enum Ask {
    /// Read a piece of a file of the mount.
    Read {
        /// The file's path inside the mount; empty for a file mount.
        path: String,
        /// Where the piece starts.
        offset: u64,
        /// How many bytes at most.
        length: u32,
    },
    /// Write a piece of a file of the mount, in place.
    Write {
        /// The file's path inside the mount; empty for a file mount.
        path: String,
        /// Where the piece lands.
        offset: u64,
        /// The piece, verbatim.
        bytes: Bytes,
    },
    /// List a directory of the mount.
    List {
        /// The directory's path inside the mount; empty for the root.
        path: String,
    },
    /// Remove a file or an empty directory of the mount.
    Remove {
        /// The entry's path inside the mount.
        path: String,
    },
    /// Rename an entry within the mount.
    Rename {
        /// Where the entry is.
        from: String,
        /// Where it goes.
        to: String,
    },
    /// Make a directory in the mount.
    Mkdir {
        /// The directory's path inside the mount.
        path: String,
    },
    /// What an entry of the mount is: kind, size, mode, owner, group
    /// and times.
    Stat {
        /// The entry's path inside the mount; empty for the mount
        /// itself.
        path: String,
    },
    /// Set a file of the mount to a length.
    Truncate {
        /// The file's path inside the mount; empty for a file mount.
        path: String,
        /// The length after.
        size: u64,
    },
    /// Set some of an entry's attributes.
    Setattr {
        /// The entry's path inside the mount; empty for the mount
        /// itself.
        path: String,
        /// Which, and to what.
        attrs: Attrs,
    },
}

impl From<Frame<'_>> for Ask {
    fn from(frame: Frame<'_>) -> Self {
        match frame {
            Frame::Read(ask) => Ask::Read {
                path: ask.path.to_owned(),
                offset: ask.offset,
                length: ask.length,
            },
            Frame::Write(ask) => Ask::Write {
                path: ask.path.to_owned(),
                offset: ask.offset,
                bytes: Bytes::copy_from_slice(ask.bytes),
            },
            Frame::List(ask) => Ask::List {
                path: ask.path.to_owned(),
            },
            Frame::Remove(ask) => Ask::Remove {
                path: ask.path.to_owned(),
            },
            Frame::Rename(ask) => Ask::Rename {
                from: ask.from.to_owned(),
                to: ask.to.to_owned(),
            },
            Frame::Mkdir(ask) => Ask::Mkdir {
                path: ask.path.to_owned(),
            },
            Frame::Stat(ask) => Ask::Stat {
                path: ask.path.to_owned(),
            },
            Frame::Truncate(ask) => Ask::Truncate {
                path: ask.path.to_owned(),
                size: ask.size,
            },
            Frame::Setattr(ask) => Ask::Setattr {
                path: ask.path.to_owned(),
                attrs: ask.attrs,
            },
        }
    }
}
