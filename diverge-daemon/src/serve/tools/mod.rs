//! The requests over tools, served: the fifteen over the records and
//! the dependencies — [`create`], [`register`], [`get`], [`list`],
//! [`edit`], [`delete`], [`tag`], [`untag`], [`attach`], [`detach`],
//! [`connect`], [`download`], [`upload`], [`transfer`], [`filetree`] —
//! and [`templates`], what tools are made from. What a request names
//! is [`resolve`]d to a [`Found`]: a record, a transient dependency
//! that runs now, or a service, which the reading requests reach and
//! every changing one refuses with [`READ_ONLY`], since a dependency
//! is its agents'; whether the caller's grants [`reaches`] any is one
//! test. Whether a record is active, and the id of its container
//! while it runs, are [`active`] and [`running`], false and none
//! until containers run; what it is attached to is [`agents_of`], and
//! a service's attachments and owner [`service_agents`] and
//! [`service_owner`]; and [`report`] is one record as a list reports
//! it, its attachments folded in, [`report_dependency`] one transient
//! dependency from its run, [`report_service`] one service from its
//! row.

mod active;
mod attached;
mod report;
mod resolve;
mod service;

pub use active::*;
pub use attached::*;
pub use report::*;
pub use resolve::*;
pub use service::*;

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
