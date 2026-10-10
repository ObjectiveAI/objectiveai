//! The requests over tools, served: the fifteen over the records and
//! the dependencies — [`create`], [`connect`], [`get`], [`list`],
//! [`edit`], [`delete`], [`tag`], [`untag`], [`attach`], [`detach`],
//! [`expose`], [`download`], [`upload`], [`transfer`], [`filetree`] —
//! and [`templates`], what tools are made from. What a request names
//! is [`resolve`]d to a [`Found`]: a record, or a dependency that
//! runs now, which the reading requests reach and every changing one
//! refuses with [`READ_ONLY`], since a dependency is its agent's;
//! whether the caller's grants [`reaches`] either is one test.
//! Whether a record is active, and the id of its container while it
//! runs, are [`active`] and [`running`], false and none until
//! containers run; what it is attached to is [`agents_of`]; and
//! [`report`] is one record as a list reports it, its attachments
//! folded in, [`report_dependency`] one dependency from its run.

mod active;
mod attached;
mod report;
mod resolve;

pub use active::*;
pub use attached::*;
pub use report::*;
pub use resolve::*;

pub mod attach;
pub mod connect;
pub mod create;
pub mod delete;
pub mod detach;
pub mod download;
pub mod edit;
pub mod expose;
pub mod filetree;
pub mod get;
pub mod list;
pub mod tag;
pub mod templates;
pub mod transfer;
pub mod untag;
pub mod upload;
