//! Images — what a provider can supply, and what a caller keeps on
//! it.
//!
//! An image is the ARTIFACT, not a running container. [`read`] asks
//! whether a provider could supply one without the caller's help —
//! from the caller's Store, or from a registry it is configured with
//! — which settles nothing about execution; it settles whether
//! execution is possible at all.
//!
//! # The Store
//!
//! The other six are the Store: the images a caller owns on the
//! provider, each by its digest alone, with a description the caller
//! may give it. [`list`] is the listing, kept open; [`stat`] examines
//! one; [`build`] makes one from a Containerfile, streaming what the
//! build says as it goes; [`push`] takes one already built, from a
//! registry the provider uses or from the caller; [`edit`] changes
//! its description; [`delete`] destroys one. The rules:
//!
//! 1. **By digest.** An image in the Store is named by its manifest
//!    digest and by nothing else. A caller owns a digest once.
//! 2. **The caller's own.** The Store is the caller's: no other
//!    identity can list, examine, edit, pull, run from or delete what
//!    the caller owns. Two callers owning one digest each own it.
//! 3. **Ownership is proven when it is gained.** A build proves it by
//!    building. A push from a registry takes the registry's bytes,
//!    verified against the digest; a push from the caller ends with
//!    the nonce check of
//!    [`oci::prove`](crate::shared::containers::oci::prove), whether
//!    or not a byte moved.
//! 4. **One copy per blob.** How the provider keeps the bytes is its
//!    own, and a blob several owners' images reach is the same blob;
//!    a caller's delete removes its ownership, and the bytes go when
//!    nothing reaches them.
//!
//! # No eviction, and a capacity
//!
//! The Store has a capacity of the provider's, apart from whatever
//! cache the provider keeps of images it pulls, and nothing a caller
//! owns is evicted: an image stays until the identity deletes it, and
//! a build or a push the Store has no room for is refused as a volume
//! creation is — with an answer, not an error. What the capacity is,
//! and whether it is one identity's or every identity's, is the
//! provider's.
//!
//! # An image owned is an image held, for its owner
//!
//! A [`read`] scope opened by the identity that owns an image, naming
//! its digest, is held; a run by that identity naming it runs it from
//! the Store, with nothing pulled, nobody asked, and no nonce check —
//! its answer's source is
//! [`Store`](crate::shared::containers::response::Source::Store). To
//! any other identity the image is not there: a `read` scope is
//! answered as if the Store held nothing of the kind, and a run looks
//! everywhere else — unless that identity owns the same digest
//! itself.
//!
//! # Reading what the provider holds
//!
//! The seventh, [`read`], is the question whether the provider can
//! supply one image, and, where it can, everything read of it: a
//! caller opens a scope on the provider naming one image, is answered
//! held or not, and asks, on channels, for that image's manifests,
//! blobs and proof — the run's asks the other way round — and for its
//! `/DIVERGE.md`, read out of the image without running it. All of it
//! comes from the caller's Store, or from a registry the provider is
//! configured with that one of the image's references names; never
//! another identity's Store, nor what the provider merely pulled. It
//! is how a caller with images on one provider supplies them to a run
//! on another, and the one way a caller learns what a container of an
//! image takes.
//!
//! [`refusal`] is what the handlers answer on their own: a digest the
//! caller owns no image by, and a build that ended without a result.

pub mod build;
pub mod delete;
pub mod edit;
pub mod list;
pub mod read;
pub mod push;
pub mod stat;

pub mod refusal;
