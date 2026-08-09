//! Where a definition was fetched from.

use serde::{Deserialize, Serialize};

/// A resolved remote path — always carries a concrete commit, so it
/// names one immutable version of one definition.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "remote", rename_all = "snake_case")]
pub enum RemotePath {
    /// A GitHub repository.
    Github {
        owner: String,
        repository: String,
        commit: String,
    },
    /// The connected client's own storage.
    Client {
        owner: String,
        repository: String,
        commit: String,
    },
}
