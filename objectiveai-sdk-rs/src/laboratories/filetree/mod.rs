//! Laboratory filesystem watch: the `/filetree` SSE wire types and a
//! materialized client.
//!
//! The [wire types](wire) are the shared shapes the in-container
//! `objectiveai-mcp-laboratory` binary emits over its `/filetree` SSE
//! endpoint — and that the CLI daemon re-emits verbatim on its
//! `/laboratories/{id}/filetree` proxy (the laboratory host pushes
//! every lab event up to the daemon, which folds a materialized tree
//! and serves the identical snapshot-then-deltas stream). [`FileTree`]
//! is the client that folds either into a self-updating tree —
//! construct via [`FileTree::laboratory`] or [`FileTree::daemon`].

mod wire;

pub use wire::*;

#[cfg(feature = "http")]
mod client;

#[cfg(feature = "http")]
pub use client::*;
