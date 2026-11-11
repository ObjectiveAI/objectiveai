//! The identities, as the store hands them out.

use crate::store::Store;

/// The store's identities: every operation on them is a method of
/// this.
#[derive(Debug, Clone, Copy)]
pub struct Identities<'a> {
    pub(super) store: &'a Store,
}

impl Store {
    /// The identities.
    pub fn identities(&self) -> Identities<'_> {
        Identities { store: self }
    }
}
