//! Function execution request and response types.
//!
//! Function executions run a Function with a Profile against provided input
//! data. Supports four combinations of remote/inline Functions and Profiles:
//!
//! - Remote Function + Remote Profile (reference both by GitHub repo)
//! - Remote Function + Inline Profile
//! - Inline Function + Remote Profile
//! - Inline Function + Inline Profile

pub mod request;
pub mod response;
mod retry_token;

pub use retry_token::*;

#[cfg(feature = "http")]
mod http;

#[cfg(feature = "http")]
pub use http::*;
