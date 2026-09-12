//! The `auth` section of `config.yaml`.
//!
//! What a peer that dials the provider may present, and how each
//! presentation is judged: the [`Unbrokered`] credentials, each a
//! fixed key or a [hook](crate::hook) that judges for itself. [`Auth`]
//! holds them.
//!
//! Its own files are flattened into it, so everything is named
//! through this module and not through the file it lives in.

mod auth;
mod unbrokered;

pub use auth::*;
pub use unbrokered::*;
