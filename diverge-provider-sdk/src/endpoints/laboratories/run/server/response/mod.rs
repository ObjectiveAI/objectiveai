//! What a provider sends back on a laboratory run.
//!
//! The container's id, its filesystem, and a connector leaving. In no
//! particular order — see [`Frame`].
//!
//! Every path in the tree is relative to the container's root. What a
//! provider puts in it is its own to decide, and mounted directories
//! are the case worth knowing about: a mount is somebody else's
//! filesystem reached across a boundary that carries no change
//! notifications, so a provider that included one would be promising
//! updates it cannot deliver.
//!
//! Nothing is aliased here. The filetree frames are WRAPPED rather
//! than re-exported, because a run answers with more than a
//! filetree — and a type that is only sometimes what it points at is
//! not an alias.

mod frame;

pub use frame::*;
