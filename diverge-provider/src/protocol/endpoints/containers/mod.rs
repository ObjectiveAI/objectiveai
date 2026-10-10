//! The container scopes, served: the two runs, each a family of one
//! shared machinery, and the serve of a running container.
//!
//! `run` is what the two runs share — bringing a container up,
//! relaying what it asks, serving what the caller opens — written
//! once; [`agents`] and [`tools`] are the two families over it, and
//! [`serve`] reaches into a running container of either for its
//! runner.

pub(crate) mod run;

pub mod agents;
pub mod serve;
pub mod tools;
