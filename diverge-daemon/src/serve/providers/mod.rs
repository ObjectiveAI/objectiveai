//! The four requests over providers and the remote daemons, served.
//!
//! Both are the configuration's, and nothing here adds, edits or
//! deletes either: [`get`] and [`list`] report the providers the
//! configuration accepts, each with where the daemon stands with it
//! now; [`daemons`] reports the remote daemons it reaches through
//! them. Each the shape [`serve`](super) states, with `connected`
//! taken from what the daemon holds now.

pub mod daemons;
pub mod get;
pub mod list;
