//! What every movement of files is made of: a stream of pieces, and
//! the one rule for the paths a request names.
//!
//! [`Pieces`] is a file's bytes as they arrive or are read, piece by
//! piece, and [`pieces`] reads them off a channel the daemon opened;
//! [`validate`] and [`inside`] are the one rule for the paths a
//! request names — a component empty, `.` or `..` is none — which
//! every download, upload, transfer and filetree applies before it
//! looks. [`Error`] is a path that is not one.
//!
//! Its own files are flattened into it, so everything is named
//! through this module and not through the file it lives in.

mod error;
mod paths;
mod pieces;

pub use error::*;
pub use paths::*;
pub use pieces::*;
