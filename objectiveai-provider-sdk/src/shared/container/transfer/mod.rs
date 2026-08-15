//! Moving a file between two containers on one provider.
//!
//! A [`request::Request`] names both ends and the provider does the
//! rest. No bytes cross the wire in either direction — which is the
//! entire point, and the entire limitation.
//!
//! # Why it is not read plus write
//!
//! Because it does not have to be. A
//! [`read`](super::read) into a
//! [`write_bytes`](super::write_bytes) moves every byte out to the
//! caller and back again, twice across a network, to put a file where
//! it already was. When both containers sit on one provider that is
//! pure waste: the provider can move the file without anybody looking
//! at it.
//!
//! How far that goes is the provider's to decide and the kernel's to
//! allow. `copy_file_range(2)` keeps the copy inside the kernel with
//! no userspace round trip; on a filesystem that supports reflinks it
//! can share extents instead of duplicating them, which costs no
//! space and takes no time proportional to the file. A thirty-gigabyte
//! transfer on ten gigabytes of headroom is possible this way and is
//! not possible any other way.
//!
//! # And why read and write still exist
//!
//! Because two providers have no filesystem in common. Nothing can
//! move a file between them without the bytes travelling, so a
//! cross-provider copy is a read on one and a write on the other, with
//! the caller in between — see
//! [`write_bytes`](super::write_bytes) for why that pipes without
//! buffering.
//!
//! This is the optimisation, not the mechanism. A caller that cannot
//! use it has lost nothing but time.

pub mod request;
pub mod response;
