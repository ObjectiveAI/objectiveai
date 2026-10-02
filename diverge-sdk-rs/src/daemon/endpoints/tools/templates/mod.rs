//! Tool templates: what a tool is made from, held by its hash.
//!
//! The shape and the id are the daemon's one
//! [`template`](crate::daemon::template), led by [`ToolKind`] here,
//! the `type` `"tool"` and nothing else: everything about a tool that is not its name, not the
//! provider it runs on and not its own mounts. A caller makes one
//! with [`create`] and names it afterwards by its id, so the same
//! template made twice is one template; a [`tool`](super) is created
//! from a template by that id, with a name, a provider and mounts of
//! its own; [`list`] lists them, narrowed; [`delete`]
//! removes one no tool was made from; [`tag`] and [`untag`] change a
//! template's tags, which are the caller's and not in the template's
//! hash. An agent template and a tool template are two namespaces:
//! each family lists and deletes its own, and the `type` keeps their
//! ids apart.

mod tool_kind;
mod tool_type;

pub use tool_kind::*;
pub use tool_type::*;

pub mod create;
pub mod delete;
pub mod list;
pub mod tag;
pub mod untag;
