//! What the container's program declared when it registered.

use serde::{Deserialize, Serialize};

use crate::shared::containers::dependencies::Template;
use crate::shared::containers::secrets::Declaration;

/// The program's answer to its registration, as `Begun` carries it:
/// the dependencies it declared, each a tool container the caller
/// deploys, and the secrets it takes, each by name to its mode. Both
/// are the container's for its life. The provider carries them to
/// the caller in one
/// [`registration`](crate::shared::containers::registration) ask when
/// either is non-empty, and reads neither.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct Registered {
    /// The dependencies, in the order the program declared them: see
    /// [`dependencies`](crate::shared::containers::dependencies).
    /// Absent when empty.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub dependencies: Vec<Template>,
    /// The secrets the program takes, each by name to its mode, in
    /// the order the program declared them: see
    /// [`secrets`](crate::shared::containers::secrets). Absent when
    /// empty.
    #[serde(default, skip_serializing_if = "Declaration::is_empty")]
    pub secrets: Declaration,
}
