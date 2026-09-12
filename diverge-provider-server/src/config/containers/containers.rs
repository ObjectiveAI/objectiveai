//! The section itself.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// The `containers` section: what the running containers may reach
/// between them, and where their storage lives.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct Containers {
    /// The most memory the running containers may hold between them,
    /// in BYTES: the sum of every running container's `memory` never
    /// exceeds it, and a run that would take it over is refused.
    /// Absent means no limit of the provider's own, only the host's.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub memory: Option<u64>,
    /// The most the running containers may write between them, in
    /// BYTES: the sum of every running container's `disk` never
    /// exceeds it, and a run that would take it over is refused.
    /// Absent means no limit of the provider's own, only the host's.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub disk: Option<u64>,
    /// An ABSOLUTE path to the directory container storage is kept
    /// under: what a container writes over its image, and what holds a
    /// volume's changes apart while `persist` is `false`. A relative
    /// path is refused when the configuration is loaded. Absent means
    /// `data/containers/` under the provider's directory.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path: Option<PathBuf>,
}
