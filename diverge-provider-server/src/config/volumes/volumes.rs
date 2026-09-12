//! The section itself.

use serde::{Deserialize, Serialize};

use super::{Fixed, Store};

/// The `volumes` section: where new volumes may be created, and which
/// volumes exist already.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct Volumes {
    /// Places the provider may create a volume in, in order of
    /// preference. Empty means no client can create a volume, and
    /// `volumes::create_capacity` answers `0`.
    pub stores: Vec<Store>,
    /// Volumes that exist before any client asks, offered in a
    /// listing to every identity, or to those a hook vouches for.
    pub fixed: Vec<Fixed>,
}
