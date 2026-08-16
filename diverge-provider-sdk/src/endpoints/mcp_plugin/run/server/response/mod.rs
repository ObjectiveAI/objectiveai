//! What a provider sends back for an MCP plugin.
//!
//! One frame, once: the plugin is up, or it is not — see [`Frame`].
//!
//! The thinnest response in this specification, and deliberately so. A
//! laboratory's reports an id, a filesystem and who has left it,
//! because a laboratory is a place; a plugin is a callee, and the only
//! thing a caller needs to know before calling is whether it can.
//!
//! Nothing is aliased here, because nothing here is somebody else's
//! frame.

mod frame;

pub use frame::*;
