//! Where deliveries land: the stores the `/resource` and
//! `/continuation` routes write into, and the [`fetcher`](crate::fetcher)
//! waits on.
//!
//! Two stores with the same verbs — `chunk`, `complete`, `error`,
//! `take` — and one difference: a [`resource`] is kept in memory,
//! keyed by identity, many at once; the [`continuation`] is one
//! slot that writes to disk as chunks land.

pub mod continuation;
pub mod resource;
