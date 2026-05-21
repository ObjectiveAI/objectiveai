//! Viewer-publication subsystem.
//!
//! The HTTP client + wire types moved to
//! [`objectiveai_sdk::http::viewer`] so standalone SDK consumers can
//! use them without depending on this crate. [`Client`] here is a
//! thin wrapper that resolves `(viewer_address, viewer_signature)`
//! from a request [`crate::ctx::Context`] before delegating to the
//! SDK client.

mod client;

pub use client::*;

/// Re-export of the SDK's wire shapes. Lets `crate::viewer::request::*`
/// imports keep working after the move.
pub mod request {
    pub use objectiveai_sdk::http::viewer::*;
}
