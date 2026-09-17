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
    /// An ABSOLUTE path to the directory new volumes are created
    /// under, made when the configuration is loaded if it is not
    /// there. A relative path is refused when the configuration is
    /// loaded. On macOS the podman machine is made seeing it.
    pub path: PathBuf,
    /// The total number of bytes the provider may reserve across the
    /// volumes in this store. What `volumes::create_capacity` reports
    /// is the largest remaining room among the stores. `0` is refused
    /// when the configuration is loaded.
    pub capacity: u64,
}
