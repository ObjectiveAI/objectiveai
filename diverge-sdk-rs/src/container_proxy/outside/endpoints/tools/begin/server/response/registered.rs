//! What the container's program declared when it registered.

use serde::{Deserialize, Serialize};

use crate::shared::containers::secrets::Declaration;

/// The program's answer to its registration, as `Begun` carries it:
/// the secrets it takes, each by name to its mode, the container's
/// for its life. A tool container declares no dependencies — a list
/// its program answers with is ignored by the proxy, and none rides
/// here. The provider carries the declaration to the caller in one
/// [`registration`](crate::shared::containers::registration) ask when
/// it is non-empty, and reads nothing of it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct Registered {
    /// The secrets the program takes, each by name to its mode, in
    /// the order the program declared them: see
    /// [`secrets`](crate::shared::containers::secrets). Absent when
    /// empty.
    #[serde(default, skip_serializing_if = "Declaration::is_empty")]
    pub secrets: Declaration,
}
