//! The `/fuse/*` paths: the mounted files' reads and writes, the
//! shared vocabulary re-exported, and the executors that answer them.
//!
//! Two operations, each its own ask on `/requests` and its own answer
//! path, exactly as the vault's are:
//!
//! | ask | kind | payload after the kind | answered on | with |
//! |-----|------|------------------------|-------------|------|
//! | [`read`] | `12` | `[id…]` | `/fuse/read/{channel}` | one [`read::response::Frame`] |
//! | [`mod@write`] | `13` | `[id_len: u16 BE][id…][bytes…]` | `/fuse/write/{channel}` | one [`write::response::Frame`] |
//!
//! The answer is one message, raw, then the close. The shapes are
//! [`shared::containers::fuse`](crate::shared::containers::fuse)'s,
//! and that module says what the id is and what a file's size may
//! be.
//!
//! # Who asks
//!
//! The proxy itself, on behalf of the file it mounted: the mounts it
//! makes at its start are the
//! [`filesystem::Mount`](super::filesystem::Mount)s the server named,
//! each carrying its id and whether it is read-only; every open of
//! such a file is one [`read`] ask, every changed close one
//! [`mod@write`], and a read-only file never asks a write at all. The
//! program beside the proxy never asks these directly — it opens the
//! file.

pub use crate::shared::containers::fuse::*;

// Each operation is a module of its own here, shadowing the shared
// one it re-exports, so the executor that answers it can live under
// it.
pub mod read;
pub mod write;
