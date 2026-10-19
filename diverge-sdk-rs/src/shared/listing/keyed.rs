//! What tells one listed item from another.

/// A listed record's key: what tells it from every other item of its
/// list, as the record carries it on the wire — and the one thing a
/// [`Listing`](super::Listing) matches by, so that a `Changed` lands
/// on the item it is about and a `Removed` takes out the right one.
/// A key is the members that never change while the item is listed;
/// a member an edit may change is not in it. Each record states its
/// own, beside its definition:
///
/// - an account: its credential's identity, else its name — an
///   account has one or the other;
/// - a role, a daemon record, a vault key, a provider's volume, an
///   image: its name;
/// - an outgoing provider: its address; an incoming one: its
///   credential's identity;
/// - an agent: its template and its index; an agent template or a
///   standalone tool template: its id;
/// - a standalone tool: its origin, less the members that may change,
///   as `OriginKey`;
/// - a dependency tool: its template's id and its owner;
/// - a volume: its provider and its name, as `reference::Volume`;
/// - a postgres connection, an event: the daemon's number for it.
pub trait Keyed {
    /// The key, owned: compared, never read into.
    type Key: Eq;

    /// This item's key.
    fn key(&self) -> Self::Key;
}
