//! The six requests over the vault, served.
//!
//! One handler per request, each the shape [`serve`](super) states;
//! the twins of [`roles`](super::roles), with a key that never
//! changes and, for live state, which of its slots a running
//! container holds — a hold taken at a run's activation and dropped
//! at its inactive or its end, each a change of the key. A key's
//! users — the agents and tools binding it — are the records', read
//! with the key.

pub mod delete;
pub mod get;
pub mod list;
pub mod insert;
pub mod tag;
pub mod untag;
