//! HTTP client for the ObjectiveAI API.
//!
//! This module provides the HTTP client implementation for making requests
//! to the ObjectiveAI API. It supports both unary (request-response) and
//! streaming (Server-Sent Events) communication patterns.
//!
//! # Feature Flag
//!
//! This module is only available when the `http` feature is enabled (default).
//!
//! # Components
//!
//! - [`HttpClient`] - The main client for making API requests
//! - [`HttpError`] - Error types for HTTP operations

mod client;
mod error;

pub use client::*;
pub use error::*;
