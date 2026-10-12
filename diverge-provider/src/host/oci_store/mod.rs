//! What this provider holds of an image for a caller: the protocol's
//! `OciStore`, supplied here from the caller's pool in the client
//! image store.
//!
//! # What is served
//!
//! The caller's own images, built on this provider and exported once
//! at the build into the caller's pool — see
//! [`image_manager`](crate::host::image_manager) — served byte for
//! byte: a has is the caller's record by digest, answered with the
//! repository path the provider's own registry serves the image at;
//! a manifest is the pool's blob under a digest the caller's records
//! name, with the media type it names for itself; and a blob is the
//! pool's blob under its digest. Nothing of another identity's is
//! served, since only the caller's own records and pool are looked
//! in, and the pool is named by the caller's identity.
//!
//! # What is not
//!
//! An image the provider merely pulled. Podman keeps a pulled image's
//! layers uncompressed and recompresses them on the way out, so what
//! leaves the cache does not hash to the digests the image is known
//! by, and nothing could be served under them. A has for such a
//! digest answers not held, and the asking side pulls it where it
//! came from, by its references.
//!
//! There is no error vocabulary: a pool that cannot be read is an
//! image not held, which the handler sends as the empty finish.
//!
//! Its own files are flattened into it, so everything is named
//! through this module and not through the file it lives in.

mod oci_store;

pub use oci_store::*;
