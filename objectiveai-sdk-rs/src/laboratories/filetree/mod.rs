//! Laboratory filesystem watch: the `/filetree` SSE wire types and a
//! materialized client.
//!
//! The [wire types](wire) are the shared shapes the in-container
//! `objectiveai-mcp-laboratory` binary emits over its `/filetree` SSE
//! endpoint. [`FileTree`] is the client that folds them into a
//! self-updating tree.

mod wire;

pub use wire::*;

#[cfg(feature = "http")]
mod client;

#[cfg(feature = "http")]
pub use client::*;
