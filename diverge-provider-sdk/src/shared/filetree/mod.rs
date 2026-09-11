//! Filetree — a provider's view of a filesystem, and how it stays
//! live.
//!
//! A caller names a root and gets its tree, then keeps getting the
//! changes to it. The shape is snapshot-then-deltas: one full tree up
//! front, then one frame per changed node, indefinitely. No polling;
//! the full tree is sent again only when the source lost track of
//! the changes, and then whole.

pub mod response;
