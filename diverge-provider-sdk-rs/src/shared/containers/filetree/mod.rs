//! Watching a container's filesystem.
//!
//! The caller opens a channel that carries nothing — the channel is
//! the ask, and there is no request type — and the provider answers
//! with a
//! [`filetree`](crate::shared::filetree) stream over the container's
//! root: one snapshot, then one frame per change, for as long as the
//! channel lives. Every path is relative to the container's root. The
//! FUSE mounts, which the caller serves itself, are left out of the
//! tree, and so is every mount of a volume whose listing says it is
//! not; every other volume mount is in it, and a change under one is
//! reported.
//!
//! A channel rather than the main stream, so a caller that wants no
//! tree pays for none, and two callers on one container can each
//! watch on their own terms. Every channel is a fresh subscription,
//! starting whole.

pub mod response;
