//! Profile management and computation types.
//!
//! - [`response`] - Profile listing and usage responses
//! - [`computations`] - Profile training/computation request and response types

pub mod computations;
pub mod response;

#[cfg(feature = "http")]
mod http;

#[cfg(feature = "http")]
pub use http::*;
