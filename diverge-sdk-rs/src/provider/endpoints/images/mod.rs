//! Images — what a provider can supply, and what a caller builds on
//! it.
//!
//! An image is the ARTIFACT, not a running container. [`check`] asks
//! whether a provider could supply one without the caller's help,
//! which settles nothing about execution; it settles whether
//! execution is possible at all.
//!
//! # The caller's own images
//!
//! The other four are a store: images a caller BUILDS on the
//! provider, from a Containerfile the request carries, each under a
//! name the caller chose, listed, examined and deleted as volumes
//! are. [`list`] is the listing, kept open; [`stat`] examines one;
//! [`build`] makes one, streaming what the build says as it goes;
//! [`delete`] destroys one. The store is the caller's: a name is
//! unique within the identity that built it, and no other identity
//! can list, examine, run or delete it.
//!
//! # No eviction, and a capacity
//!
//! The store has a capacity of the provider's, apart from whatever
//! cache the provider keeps of images it pulls, and nothing in it is
//! evicted: an image stays until the identity deletes it, and a build
//! the store has no room for is refused as a volume creation is —
//! with an answer, not an error. What the capacity is, and whether
//! it is one identity's or every identity's, is the provider's.
//!
//! # An image built is an image held, for its builder
//!
//! A [`check`] by the identity that built an image, naming its
//! digest, is available; a run by that identity naming it runs it
//! from the store, with nothing pulled and nobody asked. To any other
//! identity the image is not there: a check is answered as if the
//! store held nothing of the kind, and a run looks everywhere else —
//! unless that identity built the same image itself, in which case
//! it is its own. The store is a source for its builder and for
//! nobody else.
//!
//! [`refusal`] is what the handlers answer on their own: a name the
//! caller has no image by, and a build that ended without a result.

pub mod build;
pub mod check;
pub mod delete;
pub mod list;
pub mod stat;

pub mod refusal;
