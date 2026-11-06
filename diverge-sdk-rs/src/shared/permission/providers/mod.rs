//! What a grant over providers allows.
//!
//! The daemon's providers are its records: a grant over them names the
//! acts it allows — reading them, discovering through them, and adding,
//! editing, deleting, enabling, disabling and tagging them.
//!
//! Its own files are flattened into it, so everything is named
//! through this module and not through the file it lives in.

mod action;
mod permission;

pub use action::*;
pub use permission::*;
