//! Reading one file off the daemon's host.
//!
//! A client names a file by its absolute host path; the daemon
//! streams the file's bytes back and finishes. Split by who SENDS, as
//! everywhere else: the ask is in [`client`], the answer in
//! [`server`].
//!
//! One file, never a directory, for the reason a container's
//! [`read`](diverge_provider_sdk::shared::containers::read) gives:
//! what a directory holds is what a [`filetree`](super::filetree)
//! says. The file is the host's, and the host may be writing it: the
//! bytes are what the daemon read, and a file that changes underneath
//! the read is read as it changes, as a container's file is.

pub mod client;
pub mod server;
