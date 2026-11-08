//! The Store: the images callers own on this provider, the protocol's
//! `ImageManager`, supplied here over one set of blobs for every
//! owner.
//!
//! # What the Store is on disk
//!
//! The `images` section names a directory; see [`Store`] for its
//! layout, its reachability rules and its capacity. In short:
//! `blobs/<algorithm>/<hex>` holds every blob ONCE, however many
//! owners' images reach it; `owners/<path_name(issuer)>/<path_name(subject)>/<algorithm>-<hex>.json`
//! is one identity's ownership of one image, by its manifest digest,
//! with its description, its size, when it was gained and — for an
//! image built here — its Containerfile; `cache/<algorithm>-<hex>.json`
//! is an image a caller supplied to a run and proved, which nobody
//! owns; `tmp/` is a blob on its way in. A blob is kept while an
//! ownership, a cache record or an attempt in flight reaches it, and
//! a prune removes every other.
//!
//! # How an image is owned
//!
//! 1. **Build.** `podman build` in the cache, exported once as an OCI
//!    layout and gathered into `blobs/` — see [`build`]. The
//!    manifest's digest is the image's from then on. A digest the
//!    caller owns already is `Exists`.
//! 2. **Push.** From a registry the provider uses, through the
//!    [`RegistryClient`](crate::host::registry_client::RegistryClient),
//!    or from the caller, ending with the nonce check — see [`push`].
//!    Only what the Store lacks moves.
//!
//! Ownership is the record; nothing else is. A delete removes the
//! caller's record and prunes; an edit rewrites its description; a
//! hash is [`Store::prove`] over the Store's own bytes, the function
//! the nonce check of a run and of a push uses.
//!
//! # The nonce check
//!
//! [`fetch::check`] draws 32 bytes from the operating system's
//! cryptographic source for one check, asks the caller for
//! `SHA-256(nonce ‖ D)` — `D` the manifest, the platform manifest when
//! the manifest is an index, the config, then every layer in order,
//! duplicates included — and compares it with [`Store::prove`] over
//! the provider's own copy. Only two equal values pass. The platform
//! of an index is the one [`manifest::select`] takes: `linux` on this
//! host's architecture, the manifest podman pulls on the same host.
//!
//! # Capacity
//!
//! Every blob together holds at most `capacity` bytes, each counted
//! once. A push is refused before any blob moves, from the sizes the
//! manifest declares; a build is measured after, since its size is
//! not known before. Room is made by trimming cache records, oldest
//! first; an ownership is never trimmed.
//!
//! [`ImageManager`] is the manager, [`Store`] the Store the manager,
//! the registry, the deployer, the OCI store and the checker share,
//! [`Meta`] an ownership's record and [`Cached`] a cache record,
//! [`manifest`] a manifest read, [`fetch`] an image brought in from a
//! supplier, [`Pieces`] a file's bytes piece by piece, and [`Error`]
//! what the manager fails with. With no `images` section there is no
//! Store for callers: a build or a push is [`Error::Absent`], a
//! listing is empty, and nothing is owned by anyone; the deployer
//! keeps a Store of its own under the provider's `run/` with no room,
//! for the blobs a run takes from the caller.
//!
//! Its own files are flattened into it, so everything is named
//! through this module and not through the file it lives in.

mod build;
mod error;
pub mod fetch;
mod image_manager;
pub mod manifest;
mod pieces;
mod push;
mod store;

pub use build::*;
pub use error::*;
pub use image_manager::*;
pub use pieces::*;
pub use push::*;
pub use store::*;
