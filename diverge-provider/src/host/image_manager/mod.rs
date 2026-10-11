//! The images a caller builds on this provider: the protocol's
//! `ImageManager`, supplied here over a podman storage root of the
//! store's own.
//!
//! # What a client image is on disk
//!
//! The `images` section names a directory, and under it the store
//! keeps `root/`, a podman storage root apart from the cache the
//! deployer trims, and `meta/`. A build runs `podman build` against
//! `root/`, with the Containerfile the request carried written under
//! `run/builds/` and, as the context, a directory of one of the
//! caller's volumes loop-mounted as a run would mount it — or an
//! empty directory, when the request named none — and tags what it
//! built `localhost/diverge/<identity>/<name>`, the identity hashed,
//! so the root's names are one caller's apart from another's. Beside
//! it, `meta/<identity>/<name>.json` keeps what podman does not: the
//! Containerfile, the digest and the size podman answered, and when
//! the build ended. The listing is that directory read; a stat is
//! the file; a delete is the image removed from the root and the
//! file removed.
//!
//! # The capacity, and no eviction
//!
//! The store holds at most `capacity` bytes, counted as the cache is
//! counted — every image in the root, every layer, shared or not —
//! and measured after a build, since an image's size is not known
//! before it is built: a build that leaves the root over the
//! capacity is removed again and answered insufficient capacity.
//! Nothing else removes an image; the cache's trim never sees the
//! root.
//!
//! # Running one
//!
//! A run by the identity that built an image, naming its digest,
//! finds it here first — see the deployer — and copies it into the
//! cache, saved from the root and loaded under the same name, where
//! the cache may evict the copy and the root keeps the image. A copy
//! is named under `localhost/diverge/` and nothing else, which is how
//! the cache's lookup by digest passes it over: another identity's
//! run, and the `server` source of a check, never see it, and the
//! `client` source of a check reads the store for the identity
//! asking alone. A delete of an image a running container of the
//! caller runs — the copy, in use — is refused as in use.
//!
//! # Names
//!
//! A client image's name is one segment of a repository path: lower
//! case letters and digits with single `.`, `_`, `__` or `-` runs
//! between them, and no `/`, since it is a segment of the tag the
//! image is built under and the file its record is kept in. The
//! identity is joined onto the path as the volume store joins it:
//! whatever authenticated the connection, in the operator's trust.
//!
//! [`ImageManager`] is the manager, [`Store`] the store it and the
//! deployer and the checker share, [`Meta`] an image's record, and
//! [`Error`] what the manager fails with. With no `images` section
//! there is no store: a build is [`Error::Absent`], a listing is
//! empty, and nothing is held for anyone.
//!
//! Its own files are flattened into it, so everything is named
//! through this module and not through the file it lives in.

mod build;
mod error;
mod image_manager;
mod store;

pub use build::*;
pub use error::*;
pub use image_manager::*;
pub use store::*;
