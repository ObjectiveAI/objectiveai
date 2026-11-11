//! The signing keys' records, as the store hands them out.

use crate::store::Store;

/// The store's signing-key records: every operation on them is a
/// method of this.
#[derive(Debug, Clone, Copy)]
pub struct Keys<'a> {
    pub(super) store: &'a Store,
}

impl Store {
    /// The signing keys' records.
    pub fn keys(&self) -> Keys<'_> {
        Keys { store: self }
    }
}
