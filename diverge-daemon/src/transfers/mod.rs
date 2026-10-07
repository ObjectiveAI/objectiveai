//! A file or a directory copied out of a source into a destination,
//! on the daemon's own connections: the bytes never reach the client.
//!
//! A [`Source`] is an opened container, a volume, or a resource held;
//! what is at its path is a file, landing at the destination path
//! replaced whole, or a directory, landing file by file at the
//! destination joined with each file's path within it, nothing
//! removed. A destination is a path in an agent's or a tool's
//! container — started or joined for the operation — a path in a
//! volume, or a new resource made of what is copied, held by its hash
//! as an upload would hold it. A volume at either end is taken for
//! the length of the copy, or the copy is `Held`; two containers on
//! one provider copy through the provider, the bytes staying there;
//! every other pair streams through the daemon. [`copy`] is the whole
//! of it, and [`Fail`] why it did not land.
//!
//! Its own files are flattened into it, so everything is named
//! through this module and not through the file it lives in.

mod copy;
mod fail;
mod keep;
mod sink;
mod source;

pub use copy::*;
pub use fail::*;
pub use keep::*;
pub use sink::*;
pub use source::*;
