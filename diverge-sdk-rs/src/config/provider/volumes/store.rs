//! One place volumes are created in.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// One place the provider creates volumes in: a directory, and how
/// many bytes the volumes in it may reserve between them.
///
/// Whether the directory is its own drive is outside this crate's
/// knowledge; a store is a directory and a number.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Store {
    /// The directory new volumes are created under, made by the
    /// provider at its start if it is not there; a relative path is
    /// resolved against the root, the directory that holds
    /// `config.yaml`. On macOS the podman machine is made seeing it.
    pub path: PathBuf,
    /// The total number of bytes the provider may reserve across the
    /// volumes in this store. What `volumes::create_capacity` reports
    /// is the largest remaining room among the stores. `0` is refused
    /// at the provider's start.
    pub capacity: u64,
}
