//! What an edit changes: the size, the persist mode, or both.

use serde::{Deserialize, Serialize};

/// The three kinds of edit.
///
/// A volume has two things a caller may change after it exists — how
/// many bytes it reserves, and whether it keeps what containers write
/// into it — and an edit changes one or the other or both at once.
/// An enum rather than two optional fields, because an edit that
/// changed nothing is not an edit and the wire should have no way to
/// spell one. On the wire it is postcard's enum: a varint
/// discriminant — `0` for [`Bytes`](Self::Bytes), `1` for
/// [`Persist`](Self::Persist), `2` for [`Both`](Self::Both) — and the
/// variant's fields after it.
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
    /// An absolute size, not a delta. A caller states what it wants
    /// the volume to be rather than how far to move it, so two edits
    /// that cross leave the volume at one of the two stated sizes
    /// rather than at their sum. This is what a listing then reports
    /// as [`Volume::bytes`](crate::endpoints::volumes::list::server::response::Volume::bytes).
    Bytes(u64),
    /// Whether the volume keeps what containers write into it from
    /// now on. Discriminant `1`.
    ///
    /// What a listing then reports as
    /// [`Volume::persist`](crate::endpoints::volumes::list::server::response::Volume::persist),
    /// and what every container that mounts the volume after gets:
    /// see the [`create`](crate::endpoints::volumes::create::client::request::Frame::persist)
    /// for what each value means. The content is untouched either
    /// way: a volume made `false` holds what it held, and a volume
    /// made `true` holds what it held, which is what it had before
    /// any run under `false`.
    Persist(bool),
    /// Both at once, or neither. Discriminant `2`.
    Both {
        /// As [`Bytes`](Self::Bytes).
        bytes: u64,
        /// As [`Persist`](Self::Persist).
        persist: bool,
    },
}
