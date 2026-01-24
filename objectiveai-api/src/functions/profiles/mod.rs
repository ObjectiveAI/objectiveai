//! Profile operations.
//!
//! Provides client and retrieval operations for Profiles, which contain
//! learned weights for Functions.

mod client;
pub mod computations;
pub mod retrieval_client;

pub use client::*;
