//! What a grant over the host's identities allows.
//!
//! The identities are the files under the root's `identities/`, each
//! one subject at one issuer, signed in on the host: no request signs
//! one in, refreshes one or removes one. A grant over them lists them,
//! by issuer and by subject, and nothing else; no token is ever
//! answered.
//!
//! Its own files are flattened into it, so everything is named
//! through this module and not through the file it lives in.

mod action;
mod permission;

pub use action::*;
pub use permission::*;
