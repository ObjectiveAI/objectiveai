//! Whether the caller holds an image, by name and digest.
//!
//! The provider opens a channel with a [`request::Request`] naming
//! the image, and the caller answers with one [`response::Frame`] —
//! one byte, held or not — then the finish; the empty finish is not
//! held. A provider that would take an image from the caller asks
//! this before it asks for anything of the image, so a caller that
//! does not hold it is not asked for a manifest it cannot give.

pub mod request;
pub mod response;
