//! Naming an agent, a tool or a volume: by its name, or once and for
//! all.
//!
//! Every request that acts on an agent or a tool of the caller's — a
//! delete, an edit, a message, a logs read, a tag, an untag, an attach,
//! a detach — names it one of two ways, and the caller chooses which on
//! each request. By its NAME, as its create or its register gave it, if
//! it gave one: the name names the agent or the tool now, and is free
//! again once that one is deleted. Or by its TEMPLATE and its INDEX,
//! the number its list item carries among all ever made from that
//! template: the pair names it once and for all, is given to nothing
//! else ever, and names it still after it is deleted — when a request
//! naming it that way finds nothing. A connected tool has no template,
//! and is named instead by the DAEMON and the TOOL it is registered
//! from — the record of the other daemon, by name, and the tool as
//! that daemon names it — which name it once and for all the same way. A
//! dependency tool has neither, and is named by the AGENT it was
//! deployed for and the TEMPLATE it was deployed from, by id, which
//! name it while it runs. These are the variants of [`Agent`] and of
//! [`Tool`], one JSON object each, told apart by their members:
//! `{"name":…}`, `{"template":…,"index":…}`, for a connected tool
//! `{"daemon":…,"tool":…}`, for a dependency tool
//! `{"agent":…,"template":…}`. An object carrying members of more
//! than one is malformed.
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
mod tool;
mod volume;

pub use agent::*;
pub use tool::*;
pub use volume::*;
