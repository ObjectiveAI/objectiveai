//! The authorization codes, as the store hands them out.

use crate::store::Store;

/// The seconds a code is good for: the client exchanges it at once.
pub const LIFETIME: i64 = 60;

/// The store's authorization codes: every operation on them is a
/// method of this.
#[derive(Debug, Clone, Copy)]
pub struct Codes<'a> {
    pub(super) store: &'a Store,
}

impl Store {
    /// The authorization codes.
    pub fn codes(&self) -> Codes<'_> {
        Codes { store: self }
    }
}
