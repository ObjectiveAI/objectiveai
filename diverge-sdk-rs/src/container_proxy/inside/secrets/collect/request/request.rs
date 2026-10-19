//! Which secrets the proxy collects.

use serde::{Deserialize, Serialize};

/// The body of `POST /secrets/collect`: the names of the secrets the
/// program is to hand back, each one it declared `rotating`, in the
/// order it declared them.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct Request {
    /// The names, as the program declared them.
    pub names: Vec<String>,
}
