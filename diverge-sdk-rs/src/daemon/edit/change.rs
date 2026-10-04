//! What an edit does to one member: remove it, or replace it.

use serde::{Deserialize, Serialize};

/// One change to one member of a container: take it away, or put
/// this in its place, whole. Every member of an [`Edit`](super::Edit)
/// is an `Option` of one of these — absent, the member is as it is —
/// so an edit says, member by member, nothing, `delete`, or `set`.
///
/// Externally tagged JSON: the string `"delete"`, or `{"set":…}` with
/// the new value. Tagged rather than flat, because the value may be a
/// string — a name — and a bare `"delete"` would be a name as well as
/// a word.
///
/// # What `delete` leaves
///
/// What a create that left the member out would have made: no name,
/// no deployer, no mount of that kind, a daemon's tool `disabled`.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Change<T> {
    /// The member is taken away.
    Delete,
    /// The member is replaced, whole, by this.
    Set(T),
}
