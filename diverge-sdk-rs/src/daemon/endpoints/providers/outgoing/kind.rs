//! A mode without its secret.

use serde::{Deserialize, Serialize};

/// Which [`Mode`](super::Mode) an outgoing provider is dialled in,
/// without the credential: the mode's name alone, what a list and a get
/// report, and what a list narrows by. Snake case on the wire:
/// `"unbrokered"`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Kind {
    /// [`Mode::Unbrokered`](super::Mode::Unbrokered).
    Unbrokered,
}
