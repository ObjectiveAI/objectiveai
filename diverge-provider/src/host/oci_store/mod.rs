//! What this provider holds of an image for a caller: the protocol's
//! `OciStore`, supplied here as nothing until the store exists.
//!
//! The protocol is in place — a pull scope is held and every ask on
//! it answered — and this provider holds nothing for anyone yet:
//! every has is not held and every manifest and blob the empty
//! finish. Serving the caller's own images and the provider's own
//! from the podman store is the next step, with the client image
//! store.
//!
//! Its own files are flattened into it, so everything is named
//! through this module and not through the file it lives in.

mod oci_store;

pub use oci_store::*;
