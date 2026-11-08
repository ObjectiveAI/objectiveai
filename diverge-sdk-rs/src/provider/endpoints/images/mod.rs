//! Images — what a provider can supply, and what a caller keeps on
//! it.
//!
//! An image is the ARTIFACT, not a running container. [`check`] asks
//! whether a provider could supply one without the caller's help,
//! which settles nothing about execution; it settles whether
//! execution is possible at all.
//!
//! # The Store
//!
//! Seven of the other eight are the Store: the images a caller owns on
//! the provider, each by its digest alone, with a description the
//! caller may give it. [`list`] is the listing, kept open; [`stat`]
//! examines one; [`build`] makes one from a Containerfile, streaming
//! what the build says as it goes; [`push`] takes one already built,
//! from a registry the provider uses or from the caller; [`edit`]
//! changes its description; [`hash`] computes the proof of possession
//! over it; [`delete`] destroys one. The rules:
//!
//! 1. **By digest.** An image in the Store is named by its manifest
//!    digest and by nothing else. A caller owns a digest once.
//! 2. **The caller's own.** The Store is the caller's: no other
//!    identity can list, examine, edit, hash, run from or delete what
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
//! A [`check`] by the identity that owns an image, naming its digest,
//! is available; a run by that identity naming it runs it from the
//! Store, with nothing pulled, nobody asked, and no nonce check — its
//! answer's source is
//! [`Store`](crate::shared::containers::response::Source::Store). To
//! any other identity the image is not there: a check is answered as
//! if the Store held nothing of the kind, and a run looks everywhere
//! else — unless that identity owns the same digest itself.
//!
//! # Pulling what the provider holds
//!
//! The ninth, [`oci`], is the run's OCI asks the other way round: a
//! caller opens a scope on the provider and asks, on channels,
//! whether the provider holds an image in the caller's Store, for a
//! manifest, for a blob — never another identity's, nor one the
//! provider merely pulled. It is how a caller with images on one
//! provider supplies them to a run on another; the proof such a run
//! asks for is [`hash`]'s, relayed.
//!
//! [`refusal`] is what the handlers answer on their own: a digest the
//! caller owns no image by, and a build that ended without a result.

pub mod build;
pub mod check;
pub mod delete;
pub mod edit;
pub mod hash;
pub mod list;
pub mod oci;
pub mod push;
pub mod stat;

pub mod refusal;
