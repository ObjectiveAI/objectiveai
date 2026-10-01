//! Upload request data.
//!
//! [`Frame`] says which kind of resource is coming and, for a
//! directory, which files it holds. The bytes themselves do not
//! travel here: the daemon asks for them.

mod frame;

pub use frame::*;
