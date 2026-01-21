//! Profile computation (training) request and response types.
//!
//! Profile computations train a Profile by running a Function against a
//! dataset of example inputs with expected outputs, optimizing the weights
//! to minimize loss.

pub mod request;
pub mod response;
mod retry_token;

pub use retry_token::*;

#[cfg(feature = "http")]
mod http;

#[cfg(feature = "http")]
pub use http::*;
