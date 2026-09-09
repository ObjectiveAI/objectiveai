//! The `/filesystem/*` paths: the server's three ways into the
//! container's filesystem.
//!
//! | path | carries |
//! |------|---------|
//! | [`/filesystem/tree`](tree) | the tree, watched: a snapshot, then every change, or why there is none |
//! | [`/filesystem/read`](read) | one file out: the server names it, the container answers its bytes, or why not |
//! | [`/filesystem/write`](mod@write) | one file in: the server names it and sends its content, the container answers ok or error |
//!
//! All three are opened by the server, as many times as it likes —
//! each a subscription, each one file — and nothing is shared
//! between openings. The shapes are the wire's own,
//! [`shared::containers`](crate::shared::containers)' `filetree`,
//! `read`, `write_path` and `write_bytes`, re-exported or wrapped by
//! each path with the one thing this wire adds: a way to say why
//! there will not be one.

pub mod read;
pub mod tree;
pub mod write;
