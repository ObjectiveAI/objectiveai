//! One ask a mount makes, owned.

use bytes::Bytes;

use super::super::super::server::channel_request::Frame;

/// What a mount asks the server for, owned, so it outlives the
/// message it arrived in and is answered whenever the caller answers.
/// No mount id: the scope the ask arrived on is the mount, and the
/// server puts the caller's id back when it relays.
#[derive(Debug, Clone)]
pub enum Ask {
    /// Read a file of the mount, whole.
    Read {
        /// The file's path inside the mount; empty for a file mount.
        path: String,
    },
    /// Write a file of the mount, whole.
    Write {
        /// The file's path inside the mount; empty for a file mount.
        path: String,
        /// The file, whole, verbatim.
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
    /// What an entry of the mount is, and how long.
    Stat {
        /// The entry's path inside the mount; empty for the mount
        /// itself.
        path: String,
    },
}

impl From<Frame<'_>> for Ask {
    fn from(frame: Frame<'_>) -> Self {
        match frame {
            Frame::Read(ask) => Ask::Read {
                path: ask.path.to_owned(),
            },
            Frame::Write(ask) => Ask::Write {
                path: ask.path.to_owned(),
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
        }
    }
}
