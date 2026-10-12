//! The registry a runtime pulls from, for an image taken from the
//! caller or from a pool of the client image store: the protocol's
//! `ImageRegistry`, supplied here.
//!
//! # A pass-through for a run, a pool for a client image
//!
//! An OCI registry, the read side of the Distribution API, on its own
//! ephemeral loopback port, serving two kinds of repository. One
//! repository per run, from the
//! [`ImageSource`](crate::protocol::image_source::ImageSource)
//! the protocol hands it — the run scope's channels to the caller:
//! what podman asks it for, it asks the caller for, and hands on. A
//! manifest is fetched whole, since a manifest is small, verified,
//! answered, and kept for the run: podman asks for it more than once
//! and an index's platform manifest follows it. A blob is never
//! whole anywhere: its pieces stream from the caller to podman as
//! they arrive, hashed on the way, with one piece held back so that a
//! blob whose hash does not come out is never completed — the
//! response breaks off, podman's own digest check fails the pull, and
//! the run fails with it. Nothing of a run touches the disk, and
//! nothing is in memory but a manifest, one piece per blob in
//! flight, and the sizes a manifest's descriptors declare, which are
//! what a blob's `Content-Length` is when it is known.
//!
//! And one repository per identity's pool, named by the identity's
//! hash, as the [`image_manager`](crate::host::image_manager) keeps
//! it: a manifest is the pool's blob under the digest, read whole; a
//! blob is the file streamed off the disk through the same hashing,
//! its length from the file; a `HEAD` on a blob the pool does not
//! hold is a `404`, since the disk can say. A pool is what a run of a
//! client image pulls from, and the same pull a caller's image takes,
//! with the disk in place of the caller.
//!
//! # What podman asks, and what it gets
//!
//! `GET /v2/` is `200`. `GET` and `HEAD` on a manifest by digest are
//! the manifest with its media type as `Content-Type`; a manifest by
//! tag is `404`, since a client image is pinned by digest and a tag
//! is never pulled. `GET` on a blob by digest is the bytes, streamed;
//! `HEAD` is the headers alone, and asks the caller nothing. A `Range`
//! is not served: the whole blob is answered with `200`, which makes
//! the runtime fall back to an ordinary download. A repository that
//! is neither a run being served nor a pool, a digest the caller or
//! the pool does not hold, and anything else — auth, tags, the
//! catalog, referrers, a push — are `404` with the Distribution API's
//! error body. The name between the repository and `manifests` or
//! `blobs` is whatever the puller wrote and is not read: the digest
//! identifies the bytes.
//!
//! [`ImageRegistry`] is the registry: started on its port, serving
//! and releasing run repositories as the protocol asks. [`Repository`]
//! is one run's source with the manifests it has answered, and
//! [`pool`] a pool answered. [`Digest`] and [`Hasher`] are the digests
//! podman speaks, [`verified`] the stream that hashes a blob on its
//! way through, [`Ask`] what a path asked for, [`router`] the HTTP
//! over a [`Registry`], and [`Error`] what starting or serving fails
//! with.
//!
//! Its own files are flattened into it, so everything is named
//! through this module and not through the file it lives in.

mod digest;
mod error;
mod image_registry;
mod path;
pub mod pool;
mod repository;
mod router;
mod stream;

pub use digest::*;
pub use error::*;
pub use image_registry::*;
pub use path::*;
pub use pool::stored;
pub use repository::*;
pub use router::*;
pub use stream::*;
