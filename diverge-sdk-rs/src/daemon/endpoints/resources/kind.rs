//! The two kinds of resource.

use serde::{Deserialize, Serialize};

/// A file, or a directory of files: what an [`upload`](super::upload)
/// states and a [`list`](super::list) reports, and what decides which
/// list of a template's mounts a resource may be on. JSON `file` or
/// `directory`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Kind {
    /// One file. Its id is its bytes' hexadecimal SHA-256.
    File,
    /// One directory of files. Its id is the `h1:` summary hash.
    Directory,
}
