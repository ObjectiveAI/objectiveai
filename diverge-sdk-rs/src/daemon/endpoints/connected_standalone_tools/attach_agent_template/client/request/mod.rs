//! Attach request data.
//!
//! What a caller hands the daemon to attach a connected tool to every
//! agent of a template: the tool and the template's id, and nothing
//! else.

mod frame;

pub use frame::*;
