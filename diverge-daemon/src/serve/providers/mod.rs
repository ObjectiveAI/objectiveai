//! The requests over providers, served.
//!
//! The providers are the configuration's, and nothing here adds, edits
//! or deletes one: [`get`] and [`list`] report the providers the
//! configuration accepts, each with where the daemon stands with it
//! now; [`discover`] lists the identities discoverable through them.

pub mod discover;
pub mod get;
pub mod list;
