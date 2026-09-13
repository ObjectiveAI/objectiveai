//! The channels a caller opens on a provider.
//!
//! The five every container scope has — leaving, the tree, a read, a
//! write, the caller's half of a database connection — and then this
//! family's own. See [`Frame`].

mod frame;

pub use frame::*;
