//! The refresh tokens, as the store hands them out.

use crate::store::Store;

/// The store's refresh tokens: every operation on them is a method of
/// this.
#[derive(Debug, Clone, Copy)]
pub struct Refresh<'a> {
    pub(super) store: &'a Store,
}

impl Store {
    /// The refresh tokens.
    pub fn refresh(&self) -> Refresh<'_> {
        Refresh { store: self }
    }
}
