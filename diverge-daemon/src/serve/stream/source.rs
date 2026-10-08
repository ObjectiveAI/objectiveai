//! Where a listing reads from.

use std::future::Future;
use std::hash::Hash;

/// A list as its handler would send it now: every item the grants
/// reach and the filter admits, judged and filtered, in the store's
/// order, each with the key it is told apart by — uncapped, since the
/// cap is the listing's.
pub trait Source {
    /// What tells one item from another: an address, an identity, a
    /// name.
    type Key: Hash + Eq + Clone;
    /// The item as the wire reports it.
    type Item: PartialEq + Clone;
    /// Why the list could not be read: the store, or whatever else
    /// the item is read from.
    type Error;

    /// The list, now.
    fn read(&self) -> impl Future<Output = Result<Vec<(Self::Key, Self::Item)>, Self::Error>> + Send;
}
