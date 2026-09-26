//! A manifest by digest.
//!
//! The provider opens a channel with a [`request::Request`] naming a
//! digest, and the caller answers with one [`response::Frame`] — the
//! manifest's media type and its bytes — then the finish, or the
//! empty finish for a digest it does not hold. One frame, because a
//! manifest is small. An index is a manifest like any other: the
//! runtime reads it and asks for its platform's manifest by digest,
//! and the provider fetches that one too.

pub mod request;
pub mod response;
