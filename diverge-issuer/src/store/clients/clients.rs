//! The clients, as the store hands them out.

use crate::store::Store;

/// The store's clients: every operation on them is a method of this.
#[derive(Debug, Clone, Copy)]
pub struct Clients<'a> {
    pub(super) store: &'a Store,
}

impl Store {
    /// The clients.
    pub fn clients(&self) -> Clients<'_> {
        Clients { store: self }
    }
}
