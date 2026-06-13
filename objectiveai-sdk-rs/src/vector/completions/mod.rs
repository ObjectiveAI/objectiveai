//! Vector completions request and response types.

pub mod request;
pub mod response;
mod vector_responses;

pub use vector_responses::*;

#[cfg(feature = "http")]
mod http;

#[cfg(feature = "http")]
pub use http::*;
