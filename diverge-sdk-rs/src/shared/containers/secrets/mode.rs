//! How a program takes one of its secrets.

use serde::{Deserialize, Serialize};

/// What a program does with a secret, declared per name when it
/// registers: read it, or hold and change it. Snake case on the
/// wire: `"fixed"`, `"rotating"`.
///
/// The mode is the program's to declare and the caller's to honour:
/// a template names the secret, a record binds the name to a vault
/// key, and this says whether the value the container is handed is
/// one of the key's to read or one of the key's to keep.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[derive(schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Mode {
    /// Read, never locked: at every activation the caller hands the
    /// container a random value among the key's, and takes nothing
    /// back at the inactive. Several containers may read the same
    /// value at once.
    Fixed,
    /// Held and written back: at every activation the caller locks
    /// one free value of the key's to the container and hands it
    /// over; at the inactive the caller collects what the program
    /// made of it, writes that into the same place and drops the
    /// lock. No two containers hold one value at once, and a key
    /// with every value held makes the next activation fail.
    Rotating,
}
