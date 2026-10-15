//! The images a caller builds on this provider: the protocol's
//! `ImageManager`, supplied here over a pool per identity, kept as a
//! registry keeps images and served by the provider's own.
//!
//! # What a client image is on disk
//!
//! The `images` section names a directory, and under it the store
//! keeps one POOL per identity, `<hash>/`, the identity as sixteen
//! hexadecimal characters of its SHA-256: `blobs/<algorithm>/<hex>`,
//! every blob of every image the identity built — manifests, configs
//! and layers, content-addressed, a blob two of its images share
//! stored once — and `images/<name>.json`, one record per image:
//! the manifest digest it is known by, its size, when the build
//! ended, the Containerfile, and every blob of its. A build runs
//! `podman build` in the cache the deployer trims, with the
//! Containerfile the request carried written under `run/builds/` and,
//! as the context, a directory of one of the caller's volumes
//! loop-mounted as a run would mount it — or an empty directory,
//! when the request named none — tagged `127.0.0.1:<port>/<hash>/<name>`,
//! the name it is pulled by; then exports it once as an OCI layout
//! and moves the layout's blobs into the pool, where they are the
//! bytes a pull serves, byte for byte, and the manifest's digest is
//! the digest the image is known by from then on: what is listed,
//! checked, run and served is one set of bytes, fixed at the build.
//! The listing is the records read; a stat is one record; a delete
//! is the record removed, the pool pruned of what no other record
//! names, and the cache's copy removed.
//!
//! # The registry serves the pool
//!
//! A pool is a repository of the provider's own registry — see
//! [`image_registry`](crate::host::image_registry) — under the
//! identity's hash, so a run of a client image is a `podman pull` of
//! `127.0.0.1:<port>/<hash>/<name>@<digest>` over the loopback, or
//! through the tunnel into the machine on the hosts with one: the
//! way a caller's own image reaches podman, and the way this one
//! does, with the disk in place of the caller. The copy that pull
//! leaves in the cache is named under `127.0.0.1:` and nothing else,
//! which is how the cache's lookup by digest passes it over: another
//! identity's run never sees it, and a check never asks the cache at
//! all. The `images::oci` scope serves the same pool to the caller,
//! blob for blob.
//!
//! # The capacity, and what is evicted
//!
//! Every pool together holds at most `capacity` bytes, every blob
//! counted once, measured after a build, since an image's size is
//! not known before it is built: a build that leaves the pools over
//! the capacity is undone — its blobs pruned, its copy removed — and
//! answered insufficient capacity. Nothing else removes anything
//! from a pool: an image stays until the identity that built it
//! deletes it. The cache's copy is another matter: it is an image in
//! the cache like any the provider pulled, trimmed when the cache is
//! over its own cap, and pulled again from the pool for the next run.
//!
//! # Names
//!
//! A client image's name is one segment of a repository path: lower
//! case letters and digits with single `.`, `_`, `__` or `-` runs
//! between them, and no `/`, since it is a segment of the reference
//! the image is pulled by and the file its record is kept in. The
//! identity is joined onto the path as the volume store joins it:
//! whatever authenticated the connection, in the operator's trust,
//! hashed so that it is a path segment and is written nowhere a pull
//! could read it.
//!
//! [`ImageManager`] is the manager, [`Store`] the store it, the
//! registry, the deployer, the OCI store and the checker share,
//! [`Meta`] an image's record, [`pool`] a pool read — a manifest and
//! a blob by digest — [`Pieces`] a file's bytes piece by piece, and
//! [`Error`] what the manager fails with. With no `images` section
//! there is no store: a build is [`Error::Absent`], a listing is
//! empty, and nothing is held for anyone.
//!
//! Its own files are flattened into it, so everything is named
//! through this module and not through the file it lives in.

mod build;
mod error;
mod image_manager;
mod pieces;
pub mod pool;
mod store;

pub use build::*;
pub use error::*;
pub use image_manager::*;
pub use pieces::*;
pub use store::*;
