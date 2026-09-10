//! The `/fuse/*` paths: the mounted files' and directories' asks,
//! the shared vocabulary re-exported, and the executors that answer
//! them.
//!
//! Seven operations, each its own ask on `/requests` and its own
//! answer path, exactly as the vault's are:
//!
//! | ask | kind | payload after the kind | answered on | with |
//! |-----|------|------------------------|-------------|------|
//! | [`read`] | `12` | `[id_len: u16 BE][id…][path…]` | `/fuse/read/{channel}` | one [`read::response::Frame`] |
//! | [`mod@write`] | `13` | `[id_len: u16 BE][id…][path_len: u16 BE][path…][bytes…]` | `/fuse/write/{channel}` | one [`write::response::Frame`] |
//! | [`list`] | `14` | `[id_len: u16 BE][id…][path…]` | `/fuse/list/{channel}` | one [`list::response::Frame`] |
//! | [`remove`] | `15` | `[id_len: u16 BE][id…][path…]` | `/fuse/remove/{channel}` | one [`remove::response::Frame`] |
//! | [`rename`] | `16` | `[id_len: u16 BE][id…][from_len: u16 BE][from…][to…]` | `/fuse/rename/{channel}` | one [`rename::response::Frame`] |
//! | [`mkdir`] | `17` | `[id_len: u16 BE][id…][path…]` | `/fuse/mkdir/{channel}` | one [`mkdir::response::Frame`] |
//! | [`stat`] | `18` | `[id_len: u16 BE][id…][path…]` | `/fuse/stat/{channel}` | one [`stat::response::Frame`] |
//!
//! The answer is one message, raw, then the close. The shapes are
//! [`shared::containers::fuse`](crate::shared::containers::fuse)'s,
//! and that module says what the id and the path are, what a file's
//! size may be, and what a file mount and a directory mount each
//! allow.
//!
//! # Who asks
//!
//! The proxy itself, on behalf of what it mounted: the mounts it
//! makes at its start are the
//! [`filesystem::Mounts`](super::filesystem::Mounts) the server
//! named, files and directories, each carrying its id and whether it
//! is read-only. A file mount asks [`stat`] for every attribute,
//! [`read`] on every open and [`mod@write`] on every changed close,
//! with an empty path. A directory mount asks all seven, with the
//! entry's path: [`stat`] for every lookup and attribute; [`list`]
//! for every listing; [`read`] and [`mod@write`] for its files;
//! [`remove`], [`rename`] and [`mkdir`] for what a program does to
//! its entries. A read-only mount never asks a mutation. The
//! program beside the proxy never asks these directly — it opens the
//! files.

pub use crate::shared::containers::fuse::*;

// Each operation is a module of its own here, shadowing the shared
// one it re-exports, so the executor that answers it can live under
// it.
pub mod list;
pub mod mkdir;
pub mod read;
pub mod remove;
pub mod rename;
pub mod stat;
pub mod write;
