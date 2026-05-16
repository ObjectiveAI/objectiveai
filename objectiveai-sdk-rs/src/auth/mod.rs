//! Authentication types for the ObjectiveAI API.
//!
//! This module provides types for API key management and authentication,
//! including creating, listing, and disabling API keys, as well as
//! managing credits and OpenRouter BYOK (Bring Your Own Key) integration.
//!
//! # API Keys
//!
//! API keys are prefixed UUIDs with the format `apk<uuid>` (e.g., `apk1234abcd...`).
//! They are used to authenticate requests to the ObjectiveAI API.
//!
//! # Credits
//!
//! Credits are the billing unit for ObjectiveAI. Users can check their
//! current balance, total purchased, and total used credits.

mod api_key;
pub mod request;
pub mod response;

pub use api_key::*;

#[cfg(feature = "http")]
mod http;

#[cfg(feature = "http")]
pub use http::*;
