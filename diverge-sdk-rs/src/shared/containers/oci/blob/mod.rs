//! A blob by digest.
//!
//! The provider opens a channel with a [`request::Request`] naming a
//! digest, and the caller answers with the blob's bytes —
//! [`response::Frame`]s of at most [`CHUNK_SIZE`](crate::CHUNK_SIZE)
//! each, every frame appending to the one blob — then the finish, or
//! the empty finish for a digest it does not hold. A config and a
//! layer are both blobs; the provider does not care which.

pub mod request;
pub mod response;
