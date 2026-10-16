//! The six requests over the vault, served.
//!
//! One handler per request, each the shape [`serve`](super) states;
//! the twins of [`roles`](super::roles), with no live state to ask
//! about and a key that never changes. A key's users — the agents
//! and tools binding it — are the records', read with the key.

pub mod delete;
pub mod get;
pub mod list;
pub mod set;
pub mod tag;
pub mod untag;
