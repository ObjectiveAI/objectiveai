//! Naming an agent, a tool, a connected tool, a dependency tool or a
//! volume: by its name, or once and for all.
//!
//! Every request that acts on an agent or a tool of the caller's — a
//! delete, an edit, a message, a logs read, a tag, an untag, an attach
//! or a detach of a tool and one agent — names it one of two ways, and
//! the caller chooses which on each request. By its NAME, as its create
//! gave it, if it gave one: the name names the agent or the tool now,
//! and is free again once that one is deleted. Or by its TEMPLATE and
//! its INDEX, the number its list item carries among all ever made from
//! that template: the pair names it once and for all, is given to
//! nothing else ever, and names it still after it is deleted — when a
//! request naming it that way finds nothing. These are the variants of
//! [`Agent`] and of [`Tool`], one JSON object each, told apart by their
//! members: `{"name":…}`, `{"template":…,"index":…}`. An object carrying
//! members of both is malformed.
//!
//! A connected tool — another daemon's standalone tool, held here by a
//! [`connected_standalone_tools::create`](crate::daemon::endpoints::connected_standalone_tools::create)
//! — is no tool of the
//! [`standalone_tools`](crate::daemon::endpoints::standalone_tools)
//! family, and no request of that family finds one. It has no template,
//! and is named by [`ConnectedStandaloneTool`]: by its NAME, as its
//! create gave it, or by the DAEMON it is on — the record of the other
//! daemon, by ID — the TOOL as that daemon names it, a [`Tool`], and
//! its INDEX among all held from that daemon's tool, which name it once
//! and for all the same way: `{"name":…}`,
//! `{"daemon":…,"tool":…,"index":…}`. A name is unique across the
//! daemon's standalone and connected tools together, so a name never
//! names one of each.
//!
//! A dependency tool — one running from a template an agent's program
//! declared — is no tool of either family, and no request of theirs
//! finds one. It is named by [`Dependency`]: the TEMPLATE it runs from,
//! by id, and its [`Owner`] — the agent, named either way, the agent
//! template, or everyone — which name it while it runs,
//! `{"template":…,"owner":…}`.
//!
//! A volume is named one way only, by [`Volume`]: the provider that
//! holds it and the name that provider lists it under,
//! `{"provider":…,"name":…}`.
//!
//! A name is optional, so the once-and-for-all forms are the ones that
//! always work: an agent or a tool made with no name is reached by them
//! and no other way.

mod agent;
mod connected_standalone_tool;
mod dependency;
mod owner;
mod tool;
mod volume;

pub use agent::*;
pub use connected_standalone_tool::*;
pub use dependency::*;
pub use owner::*;
pub use tool::*;
pub use volume::*;
