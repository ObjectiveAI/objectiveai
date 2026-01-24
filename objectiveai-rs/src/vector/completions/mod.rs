//! Vector completions request and response types.

pub mod cache;
pub mod request;
pub mod response;

#[cfg(feature = "http")]
mod http;

#[cfg(feature = "http")]
pub use http::*;
