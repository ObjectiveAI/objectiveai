//! The container scopes, served: the two runs, each a family of one
//! shared machinery, and the FUSE serve of a running container.
//!
//! `run` is what the two runs share — bringing a container up,
//! relaying what it asks, serving what the caller opens — written
//! once; [`agents`] and [`tools`] are the two families over it, and
//! [`fuse_serve`] reaches into a running container of either for
//! its runner. An image's DIVERGE.md is read on no scope here: it is
//! the image's own, [`images::read`](super::images::read).

pub(crate) mod run;

pub mod agents;
pub mod fuse_serve;
pub mod tools;
