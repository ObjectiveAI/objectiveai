//! What the container's program declared when it registered.

use serde::{Deserialize, Serialize};

use crate::shared::containers::secrets::Declaration;
use crate::shared::permission::Grant;

/// The program's answer to its registration, as `Begun` carries it:
/// the secrets it takes, each by name to its mode, and the permissions
/// it holds over the daemon, the container's for its life. A tool
/// container declares no dependencies — a list its program answers
/// with is ignored by the proxy, and none rides here. The provider
/// carries both to the caller in one
/// [`registration`](crate::shared::containers::registration) ask when
/// either is non-empty, and reads nothing of them.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct Registered {
    /// The secrets the program takes, each by name to its mode, in
    /// the order the program declared them: see
    /// [`secrets`](crate::shared::containers::secrets). Absent when
    /// empty.
    #[serde(default, skip_serializing_if = "Declaration::is_empty")]
    pub secrets: Declaration,
    /// What the program may ask of the daemon, as grants, in the order
    /// it declared them: see [`Grant`]. Absent when empty.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub permissions: Vec<Grant>,
}
