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
    /// exceeds it, and a run that would take it over is refused. What
    /// a container writes is its own layer over the image; the image
    /// is not counted here.
    pub disk: u64,
    /// The most the image cache may hold, in BYTES: the layers of
    /// every image pulled, kept for the next run of it. The provider
    /// removes images no running container uses to stay under it,
    /// and an image larger than it alone cannot be pulled.
    pub images: u64,
    /// An ABSOLUTE path to the directory container storage is kept
    /// under: what a container writes over its image, and what holds a
    /// volume's changes apart while `persist` is `false`. A relative
    /// path is refused when the configuration is loaded.
    pub path: PathBuf,
}
