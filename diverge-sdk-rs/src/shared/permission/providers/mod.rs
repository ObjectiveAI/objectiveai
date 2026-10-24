//! What a grant over providers allows.
//!
//! The daemon's providers are its configuration's, read at its start:
//! no request adds, edits or removes one. A grant over them reads
//! them, and nothing else.
//!
//! Its own files are flattened into it, so everything is named
//! through this module and not through the file it lives in.

mod action;
mod permission;

pub use action::*;
pub use permission::*;
