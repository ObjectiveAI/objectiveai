//! What the manager knows about its volumes, kept between calls.
//!
//! A tree: every identity that has asked, each holding its volumes by
//! name, each volume holding what a listing knows without looking —
//! its size and when it came into being — and, only once asked, what
//! a walk finds: how many bytes it holds and the hash of its content.
//! Nothing is read before it is asked for. An identity's names are
//! read from the stores the first time the identity is named, once;
//! a volume's walk happens the first time it is checked, and again
//! only after the volume was mounted, since a container writes and
//! nothing else does.
//!
//! [`Cache`] is the tree, [`Identity`] one identity's volumes,
//! [`Volume`] one volume, [`Walked`] what a walk found, [`Sidecar`]
//! what the store keeps beside a volume, and [`walk`] the walk. The
//! locks are one per identity and one per name, so callers of
//! different volumes never wait on each other, and two callers of
//! one volume share one walk rather than running two.
//!
//! Its own files are flattened into it, so everything is named
//! through this module and not through the file it lives in.

mod cache;
mod identity;
mod sidecar;
mod volume;
mod walk;

pub use cache::*;
pub use identity::*;
pub use sidecar::*;
pub use volume::*;
pub use walk::*;
