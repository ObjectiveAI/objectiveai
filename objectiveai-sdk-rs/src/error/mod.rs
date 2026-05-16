//! Error types for the ObjectiveAI SDK.

pub mod request;
pub mod response;
mod response_error;

pub use response_error::*;

#[cfg(feature = "http")]
mod http;

#[cfg(feature = "http")]
pub use http::*;
