//! The section itself.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use super::{Fixed, Store};

/// The `volumes` section: where new volumes may be created, and which
/// volumes exist already.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct Volumes {
    /// Places the provider may create a volume in, in order of
    /// preference. Absent, or empty, means no client can create a
    /// volume, and `volumes::create_capacity` answers `0`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stores: Option<Vec<Store>>,
    /// Volumes that exist before any client asks, offered in a
    /// listing to every identity, or to those a hook vouches for.
    /// Absent, or empty, means none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fixed: Option<Vec<Fixed>>,
}

impl Volumes {
    /// The host directories the volumes live in: every store's path
    /// and every fixed volume's, in the configuration's order. What a
    /// podman machine must be able to see, on the host that has one.
    pub fn paths(&self) -> Vec<PathBuf> {
        self.stores
            .iter()
            .flatten()
            .map(|store| store.path.clone())
            .chain(self.fixed.iter().flatten().map(|fixed| fixed.path.clone()))
            .collect()
    }
}
