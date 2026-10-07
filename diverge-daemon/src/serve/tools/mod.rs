//! The requests over tools, served: the twelve over the records —
//! [`create`], [`connect`], [`get`], [`list`], [`edit`], [`delete`],
//! [`tag`], [`untag`], [`attach`], [`detach`], [`admit`], [`unadmit`]
//! — [`routes`], the positions a tool answers at, [`list_for`], the
//! one that asks a provider rather than the records, and
//! [`templates`], what tools are made from. Whether a tool is active,
//! and the id of its container while it runs, are [`active`] and
//! [`running`], false and none until containers run; what it is
//! attached to is [`agents_of`]; and [`report`] is one tool as a list
//! reports it, its attachments, routes and admissions folded in.

mod active;
mod attached;
mod report;

pub use active::*;
pub use attached::*;
pub use report::*;

pub mod admit;
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
pub mod list_for;
pub mod routes;
pub mod tag;
pub mod templates;
pub mod transfer;
pub mod unadmit;
pub mod untag;
pub mod upload;
