//! Naming an agent, a tool, a dependency tool or a volume: by its
//! name, or once and for all.
//!
//! Every request that acts on an agent or a tool of the caller's — a
//! delete, an edit, a message, a logs read, a tag, an untag, an attach
//! or a detach of a tool and one agent — names it one of two ways, and
//! the caller chooses which on each request. By its NAME, as its create or its register gave it, if
//! it gave one: the name names the agent or the tool now, and is free
//! again once that one is deleted. Or by its TEMPLATE and its INDEX,
//! the number its list item carries among all ever made from that
//! template: the pair names it once and for all, is given to nothing
//! else ever, and names it still after it is deleted — when a request
//! naming it that way finds nothing. A connected tool has no template,
//! and is named instead by the DAEMON and the TOOL it is registered
//! from — the record of the other daemon, by name, and the tool as
//! that daemon names it — which name it once and for all the same way.
//! These are the variants of [`Agent`] and of [`Tool`], one JSON
//! object each, told apart by their members: `{"name":…}`,
//! `{"template":…,"index":…}`, for a connected tool
//! `{"daemon":…,"tool":…}`. An object carrying members of more than
//! one is malformed.
//!
//! A dependency tool — one running from a template an agent's program
//! declared — is no tool of the [`standalone_tools`](crate::daemon::endpoints::standalone_tools)
//! family, and no request of that family finds one. It is named by
//! [`Dependency`]: the TEMPLATE it runs from, by id, and its
//! [`Owner`] — the agent, named either way, the agent template, or
//! everyone — which name it while it runs,
//! `{"template":…,"owner":…}`.
//!
//! A volume is named one way only, by [`Volume`]: the provider that
//! holds it and the name that provider lists it under,
//! `{"provider":…,"name":…}`.
//!
//! A name is optional, so the once-and-for-all forms are the ones that
//! always work: an agent or a tool made with no name is reached by them
//! and no other way; a request naming a template and an index finds no
//! connected tool, and one naming a daemon and a tool finds no created
//! one.

mod agent;
mod dependency;
mod owner;
mod tool;
mod volume;

pub use agent::*;
pub use dependency::*;
pub use owner::*;
pub use tool::*;
pub use volume::*;
