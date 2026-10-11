//! What this provider holds of an image for a caller: the protocol's
//! `OciStore`, supplied here from the client image store's layouts.
//!
//! # What is served
//!
//! The caller's own images, built on this provider and exported once
//! at the build as OCI layouts — see
//! [`image_manager`](crate::host::image_manager) — served byte for
//! byte: a has is the caller's record by digest, a manifest is the
//! layout's manifest blob with the media type its index names, and a
//! blob is a file of the layout's `blobs/` found by its digest among
//! the caller's layouts. Nothing of another identity's is served,
//! since only the caller's own records and layouts are looked in.
//!
//! # What is not
//!
//! An image the provider merely pulled. Podman keeps a pulled image's
//! layers uncompressed and recompresses them on the way out, so what
//! leaves the store does not hash to the digests the image is known
//! by, and nothing could be served under them. A has for such a
//! digest answers not held, and the asking side pulls it where it
//! came from, by its references.
//!
//! There is no error vocabulary: a layout that cannot be read is an
//! image not held, which the handler sends as the empty finish.
//!
//! Its own files are flattened into it, so everything is named
//! through this module and not through the file it lives in.

mod oci_store;
mod pieces;

pub use oci_store::*;
pub use pieces::*;
