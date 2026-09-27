//! Editing what a tool mounts, by name.
//!
//! One request, one answer. A client names a tool of its own and
//! states its mounts anew — the volumes of the provider it is pinned
//! to, and the files and directories served over FUSE — and the
//! daemon answers that the tool has them, that no tool has that name,
//! that the tool is active and was left as it is, or that it failed,
//! and the scope finishes. The mounts are the one thing about a tool
//! that changes after its [`create`](super::create): its image, its
//! limits, its provider and its arguments are for its life. A tool is
//! active while its container runs, which is while any agent it is
//! attached to is.
//!
//! Split by who SENDS, as everywhere else. A client asks — so the
//! question is in [`client`] — and the daemon answers, so the answer
//! is in [`server`]. Neither side holds both halves of the exchange.

pub mod client;
pub mod server;
