//! The registry the provider's runtime pulls from: one repository per
//! pull, under a random token, served from the Store — or read
//! through to a run's caller.
//!
//! # A token per pull
//!
//! An OCI registry, the read side of the Distribution API, on its own
//! ephemeral loopback port. It serves no repository of its own: every
//! repository is a token the provider mints for ONE pull and revokes
//! when that pull ends, by the rules:
//!
//! 1. **Random.** A token is 16 bytes from the operating system's
//!    cryptographic source, as 32 lowercase hexadecimal characters:
//!    128 bits no request can guess, and nothing of any identity or
//!    image in it.
//! 2. **One pull.** The deployer mints a token before its `podman
//!    pull` and revokes it when the pull answers, whichever way. A
//!    path under a token never minted, or one revoked, is `404`; so is
//!    every path under the old per-identity pools.
//! 3. **One origin.** A token maps to exactly one [`Origin`]: the
//!    [Store](Origin::Store), whose blobs it serves and nothing else;
//!    or a [run's caller](Origin::Client), read through.
//! 4. **Read-through.** For a caller's token, a manifest or blob the
//!    Store holds is served from the Store, and the caller is not
//!    asked. Any other is fetched from the caller by digest, verified
//!    against the digest, written into the Store — pinned for the
//!    attempt the token was minted for — and then served from there.
//!    Nothing is fetched from the caller but what the provider lacks,
//!    and nothing is served that is not its digest's.
//! 5. **The name is a label.** The segments between the token and
//!    `manifests` or `blobs` are the caller's `has` answer, validated
//!    as a repository path, or `image`; they are never read.
//!
//! # What podman asks, and what it gets
//!
//! `GET /v2/` is `200`. `GET` and `HEAD` on a manifest by digest are
//! the manifest with its media type as `Content-Type`; a manifest by
//! tag is `404`, since an image is pulled by digest. `GET` on a blob
//! by digest is the file streamed through the hash; `HEAD` is the
//! headers alone, with the length. A `Range` is not served: the whole
//! blob is answered with `200`. Anything else — auth, tags, the
//! catalog, referrers, a push — is `404` with the Distribution API's
//! error body.
//!
//! [`ImageRegistry`] is the registry: started on its port, minting and
//! revoking tokens. [`Repository`] is one token's origin answered.
//! [`Digest`] and [`Hasher`] are the digests podman speaks,
//! [`verified`] the stream that hashes a blob on its way out, [`Ask`]
//! what a path asked for, [`router`] the HTTP, and [`Error`] what
//! starting fails with.
//!
//! Its own files are flattened into it, so everything is named
//! through this module and not through the file it lives in.

mod digest;
mod error;
mod image_registry;
mod path;
mod repository;
mod router;
mod stream;

pub use digest::*;
pub use error::*;
pub use image_registry::*;
pub use path::*;
pub use repository::*;
pub use router::*;
pub use stream::*;
