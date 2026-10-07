//! The content store: every resource's bytes, on disk, by its hash.
//!
//! A resource is a file or a directory of files a caller uploaded,
//! and this is where the bytes are: `<dir>/resources/<id>` — a file
//! for a file resource, a tree for a directory resource — and
//! `<dir>/resources/incoming/<uuid>/` for an upload still arriving.
//! [`receive`] reads an upload off the channels the daemon opens on
//! the scope, one per file, into an incoming directory, hashing as it
//! goes, and answers the id and the byte count; [`place`] moves the
//! incoming directory to the id's place, or throws it away when the
//! id is held already; [`files`] and [`tree`] read a held resource
//! for a download and a filetree; [`remove`] takes a held resource's
//! bytes away at its delete. [`validate`] is the one rule for the
//! paths a request names. The records of what is held — kind,
//! description, tags, creator — are the store's, not here.
//!
//! Its own files are flattened into it, so everything is named
//! through this module and not through the file it lives in.

mod error;
mod paths;
mod place;
mod read;
mod receive;
mod remove;

pub use error::*;
pub use paths::*;
pub use place::*;
pub use read::*;
pub use receive::*;
pub use remove::*;
