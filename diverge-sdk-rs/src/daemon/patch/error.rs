//! Why a patch could not be applied.

use std::fmt;

/// The operation that failed, by its place in the patch, and why.
/// Nothing was applied.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Error {
    /// Which operation, counted from `0`.
    pub index: usize,
    /// Its name, as `op` spells it.
    pub operation: &'static str,
    /// Why.
    pub kind: Kind,
}

/// Why one operation failed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Kind {
    /// The pointer is not one: no leading `/`, or an escape that is
    /// neither `~0` nor `~1`.
    Pointer {
        /// The pointer as given.
        path: String,
    },
    /// Nothing is at the path, and the operation needs something
    /// there.
    Missing {
        /// The pointer.
        path: String,
    },
    /// A token names a place in a list that is not an index of it:
    /// not a number, past its end, or `-` where only an `add` may say
    /// it.
    Index {
        /// The pointer.
        path: String,
    },
    /// The path goes into something that is neither an object nor a
    /// list.
    Container {
        /// The pointer.
        path: String,
    },
    /// What is at the path is not the value tested for.
    Test {
        /// The pointer.
        path: String,
    },
    /// A `move` would put a value inside itself.
    Descendant {
        /// Where from.
        from: String,
        /// Where to.
        path: String,
    },
    /// The whole document cannot be removed or moved.
    Root,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "#{} {}: {}", self.index, self.operation, self.kind)
    }
}

impl fmt::Display for Kind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Kind::Pointer { path } => write!(f, "`{path}` is not a JSON pointer"),
            Kind::Missing { path } => write!(f, "nothing is at `{path}`"),
            Kind::Container { path } => write!(f, "`{path}` goes into something that is neither an object nor a list"),
            Kind::Index { path } => write!(f, "`{path}` names no index of the list"),
            Kind::Test { path } => write!(f, "what is at `{path}` is not the value tested for"),
            Kind::Descendant { from, path } => write!(f, "`{from}` cannot be moved into `{path}`, which is inside it"),
            Kind::Root => f.write_str("the whole document cannot be removed or moved"),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&self.kind)
    }
}

impl std::error::Error for Kind {}
