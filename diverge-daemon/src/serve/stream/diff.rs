//! The listing now against the listing as sent.

use std::collections::HashMap;
use std::hash::Hash;

use super::Change;

/// Every difference between `known`, the listing as last sent, and
/// `now`, the listing as read: a key in `now` and not in `known` is
/// `Added`; a key in both whose item differs is `Changed`; a key in
/// `known` and not in `now` is `Removed`, with the item as last sent.
/// Added and changed in `now`'s order, then removed in `known`'s.
pub fn diff<K, I>(known: &[(K, I)], now: &[(K, I)]) -> Vec<Change<I>>
where
    K: Hash + Eq,
    I: PartialEq + Clone,
{
    let was: HashMap<&K, &I> = known.iter().map(|(key, item)| (key, item)).collect();
    let is: HashMap<&K, &I> = now.iter().map(|(key, item)| (key, item)).collect();
    let mut changes = Vec::new();
    for (key, item) in now {
        match was.get(key) {
            None => changes.push(Change::Added(item.clone())),
            Some(sent) if *sent != item => changes.push(Change::Changed(item.clone())),
            Some(_) => {}
        }
    }
    for (key, item) in known {
        if !is.contains_key(key) {
            changes.push(Change::Removed(item.clone()));
        }
    }
    changes
}
