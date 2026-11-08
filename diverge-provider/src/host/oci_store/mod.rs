//! What this provider holds of an image for a caller: the protocol's
//! `OciStore`, supplied here from the caller's Store.
//!
//! # What is served
//!
//! 1. **`has`.** A digest the caller owns is held, answered with the
//!    fixed label `image`: the name is a label for a puller's
//!    reference, never stored and never read.
//! 2. **A manifest.** One the caller's Store reaches — an image it
//!    owns, or a manifest an index it owns lists and the Store holds —
//!    with the media type it names for itself.
//! 3. **A blob.** One an image the caller owns reaches. The Store keeps
//!    one copy of a blob for every owner, so what is served is decided
//!    by the caller's own records and by nothing on disk: another
//!    identity's blob is never served, however its digest is known.
//!
//! # What is not
//!
//! An image the provider merely pulled, or holds in a cache record: no
//! identity owns it. Podman keeps a pulled image's layers uncompressed
//! and recompresses them on the way out, so nothing of its cache could
//! be served under the digests anyway.
//!
//! There is no error vocabulary: a Store that cannot be read is an
//! image not held, which the handler sends as the empty finish. There
//! is no `prove` here: a proof over a Store image is
//! [`images::hash`](diverge_sdk::provider::endpoints::images::hash)'s.
//!
//! Its own files are flattened into it, so everything is named
//! through this module and not through the file it lives in.

mod oci_store;

pub use oci_store::*;
