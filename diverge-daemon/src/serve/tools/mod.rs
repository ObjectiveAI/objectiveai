//! The requests over tools, served: the fifteen over the records —
//! [`create`], [`register`], [`get`], [`list`], [`edit`], [`delete`],
//! [`tag`], [`untag`], [`attach`], [`detach`], [`connect`],
//! [`download`], [`upload`], [`transfer`], [`filetree`] — and
//! [`templates`], what tools are made from. What a request names is
//! [`resolve`]d to a record, which is the only thing a tool reference
//! names: a dependency tool is the
//! [`dependencies`](crate::serve::dependencies) family's, and no
//! request here finds one. Whether the caller's grants [`reaches`] a
//! record is one test. Whether a record is active, and the id of its
//! container while it runs, are [`active`] and [`running`], false and
//! none until containers run; what it is attached to is [`agents_of`];
//! and [`report`] is one record as a list reports it, its attachments
//! folded in.

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
pub mod filetree;
pub mod get;
pub mod list;
pub mod register;
pub mod tag;
pub mod templates;
pub mod transfer;
pub mod untag;
pub mod upload;
