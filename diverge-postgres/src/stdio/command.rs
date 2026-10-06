//! What comes in on stdin.

use serde::{Deserialize, Serialize};

/// One line of stdin: an object with a `type`, and `shutdown` is the
/// one type there is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Command {
    /// Stop the postmaster cleanly — a fast stop, waited for — and
    /// exit with success. `{"type":"shutdown"}`.
    Shutdown,
}
