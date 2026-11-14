//! Volume FUSE serve request data. What a caller hands the daemon to
//! have a volume served: the volume, the mode it is meant to be in,
//! and the size of the layer an ephemeral one is served on.

mod frame;

pub use frame::*;
