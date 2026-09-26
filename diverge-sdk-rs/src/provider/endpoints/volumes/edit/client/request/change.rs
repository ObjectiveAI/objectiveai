//! What an edit changes: the size, the mode, or both.

use serde::{Deserialize, Serialize};

use crate::provider::endpoints::volumes::Mode;

/// The three kinds of edit.
///
/// A volume has two things a caller may change after it exists — how
/// many bytes it reserves, and its [`Mode`] — and an edit changes one
/// or the other or both at once. An enum rather than two optional
/// fields, because an edit that changed nothing is not an edit and
/// the wire should have no way to spell one. On the wire it is
/// postcard's enum: a varint discriminant — `0` for
/// [`Bytes`](Self::Bytes), `1` for [`Mode`](Self::Mode), `2` for
/// [`Both`](Self::Both) — and the variant's fields after it.
///
/// # Both, or neither
///
/// [`Both`](Self::Both) is one edit, not two: a provider that cannot
/// give the size — no room for it, or content that exceeds it —
/// changes the mode no more than the size, and answers the refusal
/// with the volume as it was in every respect. A caller that wants
/// the mode changed whatever becomes of the size sends two edits.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Change {
    /// How many bytes the volume reserves from now on. Discriminant
    /// `0`.
    ///
    /// What a listing then reports as
    /// [`Volume::bytes`](crate::provider::endpoints::volumes::list::server::response::Volume::bytes).
    /// Larger reserves more; smaller gives room back, and is refused
    /// when the volume holds more than that.
    Bytes(u64),
    /// The mode the volume is in from now on. Discriminant `1`.
    ///
    /// What a listing then reports as
    /// [`Volume::mode`](crate::provider::endpoints::volumes::list::server::response::Volume::mode),
    /// and what every container that mounts the volume after, and
    /// every serve of it after, is bound under: see [`Mode`] for what
    /// each value means. The content is untouched either way: a
    /// volume made ephemeral holds what it held, and a volume made
    /// persistent holds what it held, which is what it had before any
    /// run or serve under `ephemeral`.
    Mode(Mode),
    /// Both at once, or neither. Discriminant `2`.
    Both {
        /// As [`Bytes`](Self::Bytes).
        bytes: u64,
        /// As [`Mode`](Self::Mode).
        mode: Mode,
    },
}
