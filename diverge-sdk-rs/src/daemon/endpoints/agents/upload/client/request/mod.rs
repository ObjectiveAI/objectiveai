//! Upload request data.
//!
//! [`Frame`] says which agent, where, and for a directory which files.
//! The bytes themselves do not travel here: the daemon asks for them.

mod frame;

pub use frame::*;
