//! The requests over agents, served: the eight over the records —
//! [`create`], [`get`], [`list`], [`edit`], [`delete`], [`tag`],
//! [`untag`], [`logs`] — and [`templates`], what agents are made from.
//! Whether a loop runs in an agent is [`active`], false until
//! containers run; what is attached to one is [`tools_of`]; and
//! [`Failure`] is why a handler could not answer, the store or the
//! log. A message to an agent needs its container and comes with the
//! run.

mod active;
mod attached;
mod failure;

pub use active::*;
pub use attached::*;
pub use failure::*;

pub mod create;
pub mod delete;
pub mod edit;
pub mod get;
pub mod list;
pub mod logs;
pub mod tag;
pub mod templates;
pub mod untag;
