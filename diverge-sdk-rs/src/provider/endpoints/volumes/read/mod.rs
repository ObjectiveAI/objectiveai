//! Reading one file out of a volume.
//!
//! A caller names a volume and a file in it; a provider streams the
//! file's bytes back and finishes. Split by who SENDS, as everywhere
//! else: the ask is in [`client`], the answer in [`server`].
//!
//! # The volume at rest
//!
//! A read takes the volume to itself, as a [`stat`](super::stat)
//! does: nothing is read while any container has the volume, and no
//! run takes it while the read runs. So the bytes are the file as it
//! is, with nobody writing underneath the reader — the tear a
//! container's [`read`](crate::shared::containers::read) lives with
//! cannot happen here. A file being read cannot change, and the read
//! is refused rather than served stale.
//!
//! One file, never a directory, for the reason the container's read
//! gives: what a volume holds is what a
//! [`filetree`](super::filetree) says, one snapshot at a time.

pub mod client;
pub mod server;
