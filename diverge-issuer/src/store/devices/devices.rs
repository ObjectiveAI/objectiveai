//! The device codes, as the store hands them out.

use crate::store::Store;

/// The store's device codes: every operation on them is a method of
/// this.
#[derive(Debug, Clone, Copy)]
pub struct Devices<'a> {
    pub(super) store: &'a Store,
}

impl Store {
    /// The device codes.
    pub fn devices(&self) -> Devices<'_> {
        Devices { store: self }
    }
}
