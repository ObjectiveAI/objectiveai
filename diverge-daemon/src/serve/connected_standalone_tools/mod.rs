//! The requests over connected tools, served: the eleven over the
//! records — [`create`], [`get`], [`list`], [`edit`], [`delete`],
//! [`tag`], [`untag`], [`attach_agent_instance`],
//! [`detach_agent_instance`], [`attach_agent_template`] and
//! [`detach_agent_template`]. A connected tool is another daemon's
//! standalone tool, held here on a record of its own: no request of
//! the [`standalone_tools`](crate::serve::standalone_tools) family
//! finds one, and none here finds a standalone tool — their names
//! alone are one namespace, see
//! [`tool_names`](crate::store::tool_names). What a request names is
//! [`resolve`]d to a record; whether the caller's grants [`reaches`] it
//! is one test, over the [`facts`] of it. Whether it is active and
//! whether its connect is held are [`active`] and [`deployed`]; what it
//! is attached to is [`agents_of`] and [`templates_of`]; and [`report`]
//! is one record as a list reports it, its attachments folded in.
//! [`logs`](crate::serve::logs) serves a connected tool's log beside
//! every other kind's.

mod active;
mod attached;
mod report;
mod resolve;

pub use active::*;
pub use attached::*;
pub use report::*;
pub use resolve::*;

pub mod attach_agent_instance;
pub mod attach_agent_template;
pub mod create;
pub mod delete;
pub mod detach_agent_instance;
pub mod detach_agent_template;
pub mod edit;
pub mod get;
pub mod list;
pub mod tag;
pub mod untag;
