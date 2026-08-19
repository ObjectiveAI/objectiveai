//! What a provider sends back for an MCP plugin.
//!
//! At most one frame, and usually none: the plugin did not come up —
//! see [`Frame`].
//!
//! The thinnest response in this specification, and deliberately so. A
//! laboratory's reports an id, a filesystem and who has left it,
//! because a laboratory is a place; a plugin is a callee, and what a
//! caller does with one is call it. Whether it can is answered by
//! calling, not by being told.
//!
//! So this channel carries bad news or nothing. The scope's own finish
//! is what says the run is over.
//!
//! Nothing is aliased here, because nothing here is somebody else's
//! frame.

mod frame;

pub use frame::*;
