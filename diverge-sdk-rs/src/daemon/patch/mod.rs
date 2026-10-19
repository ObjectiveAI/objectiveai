//! A JSON Patch: the one way every record of the daemon's is edited.
//!
//! Every `edit` of the daemon's names a record and carries a
//! [`Patch`] — a list of [`Operation`]s, RFC 6902 — which the daemon
//! applies, in order, to the record's EDITABLE members as one JSON
//! document, the family's `Editable` type; the result must be that
//! type again, and is then checked and written as the family's edit
//! states. One patch carries any number of operations and is applied
//! whole or not at all: the first operation that fails fails the
//! patch, and nothing changed. [`apply`] is the application, which
//! the daemon runs on a record and the CLI on a configuration
//! document; [`Error`] is why a patch could not be applied.
//!
//! # Why a patch
//!
//! A member replaced whole cannot append to a list, and a program
//! string cannot be schemed. A patch names one place by a pointer —
//! `/fuse_file_mounts/-` for the end of a list, `/name`, `/grants/2`
//! — and says what happens there, in a form every client and every
//! schema can read.

mod apply;
mod error;
mod operation;

pub use apply::*;
pub use error::*;
pub use operation::*;
