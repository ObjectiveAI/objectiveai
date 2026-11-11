//! Create request data.
//!
//! What a caller hands the daemon to hold a tool another daemon holds,
//! under a name: the daemon, by its record, the route beyond it, the
//! tool as that daemon names it, and the name. Nothing the container is
//! made from, which is the other daemon's.

mod frame;

pub use frame::*;
