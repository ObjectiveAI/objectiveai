//! The container scopes, served: the two runs, each a family of one
//! shared machinery, and the FUSE serve of a running container.
//!
//! `run` is what the two runs share — bringing a container up,
//! relaying what it asks, serving what the caller opens — written
//! once; [`agents`] and [`tools`] are the two families over it, and
//! [`fuse_serve`] reaches into a running container of either for
//! its runner. [`info`] runs nothing: it reads an image's DIVERGE.md
//! out of the image, through the deployer.

pub(crate) mod run;

pub mod agents;
pub mod fuse_serve;
pub mod info;
pub mod tools;
