//! The section itself.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// The `containers` section: what the running containers may reach
/// between them, and where their storage lives.
///
/// Every field is required when the section is present; the section
/// as a whole is what may be absent.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Containers {
    /// The most memory the running containers may hold between them,
    /// in BYTES: the sum of every running container's `memory` never
    /// exceeds it, and a run that would take it over is refused.
    pub memory: u64,
    /// The most the running containers may write between them, in
    /// BYTES: the sum of every running container's `disk` never
    /// exceeds it, and a run that would take it over is refused.
    pub disk: u64,
    /// An ABSOLUTE path to the directory container storage is kept
    /// under: what a container writes over its image, and what holds a
    /// volume's changes apart while `persist` is `false`. A relative
    /// path is refused when the configuration is loaded.
    pub path: PathBuf,
}
