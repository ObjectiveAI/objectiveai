//! The list a consumer keeps, and the fold that keeps it current.

use serde::{Deserialize, Serialize};

use super::{Change, Keyed};

/// A list as it stands: what a consumer of a list stream holds.
///
/// A newtype rather than a named field, so the listing IS its items:
/// serde treats a one-member tuple struct as the member, and it is
/// written and read as a bare JSON list, with no wrapper object
/// naming a field that could only ever hold one thing. This is also
/// the shape a consumer materializes: applying what the stream tells
/// means editing this list, so what a consumer holds and what a
/// stream delivers are the same type.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[derive(schemars::JsonSchema)]
pub struct Listing<T>(
    /// The items, in the order the stream listed them: the daemon's
    /// lists oldest first, events by id. An empty listing is an
    /// empty list, never absent.
    pub Vec<T>,
);

impl<T: Keyed> Listing<T> {
    /// Fold one [`Change`] into this list.
    ///
    /// This is THE fold. Every holder of a materialized listing
    /// applies what a stream tells the same way, so that two
    /// consumers fed the same stream cannot disagree about what it
    /// meant. The rules:
    ///
    /// 1. [`Added`](Change::Added) and [`Changed`](Change::Changed)
    ///    are one act: the item whose key matches is replaced in
    ///    place, else the item is appended. The wire distinction is
    ///    information for the consumer; the list that results is the
    ///    same either way, which is what makes the fold indifferent
    ///    to replay — applying either twice is a no-op, and applying
    ///    one where the other was expected still lands the right
    ///    value. An event whose count was raised is `Changed` with
    ///    the whole record, so replacing it IS the increment.
    /// 2. [`Removed`](Change::Removed): the item whose key matches is
    ///    taken out; none matching is nothing.
    /// 3. [`Listed`](Change::Listed): nothing. The fold keeps no word
    ///    of whether the listing is whole, since the type is the list
    ///    and nothing else; a consumer that waits for the word reads
    ///    it off the stream before folding.
    ///
    /// Nothing here panics, and no ordering is assumed.
    pub fn push(&mut self, change: Change<T>) {
        match change {
            Change::Added(item) | Change::Changed(item) => self.place(item),
            Change::Removed(item) => self.take(&item.key()),
            Change::Listed => {}
        }
    }

    /// Replace the item with `item`'s key, or append `item`.
    fn place(&mut self, item: T) {
        let key = item.key();
        match self.0.iter().position(|held| held.key() == key) {
            Some(at) => self.0[at] = item,
            None => self.0.push(item),
        }
    }

    /// Take out the item with `key`, if it is there.
    fn take(&mut self, key: &T::Key) {
        if let Some(at) = self.0.iter().position(|held| held.key() == *key) {
            self.0.remove(at);
        }
    }
}
