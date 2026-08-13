//! Filetree — a provider's view of a filesystem, and how it stays
//! live.
//!
//! A caller names a root and gets its tree, then keeps getting the
//! changes to it. The shape is snapshot-then-deltas: one full tree up
//! front, then one frame per changed node, indefinitely. No polling,
//! and the full tree is never re-sent.

mod response;

pub use response::*;
