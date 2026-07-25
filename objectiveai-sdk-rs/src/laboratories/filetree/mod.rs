//! Laboratory filesystem watch: the `/filetree` SSE wire types.
//!
//! These are the shared shapes the in-container
//! `objectiveai-mcp-laboratory` binary emits over its `/filetree` SSE
//! endpoint — and that the CLI daemon re-emits verbatim on its
//! `/laboratories/{id}/filetree` proxy (the laboratory host pushes
//! every lab event up to the daemon, which folds a materialized tree
//! and serves the identical snapshot-then-deltas stream). The
//! materialized CLIENT lives in the daemon module —
//! [`crate::daemon::file_tree::FileTree`] (feature `daemon`).

mod wire;

pub use wire::*;
