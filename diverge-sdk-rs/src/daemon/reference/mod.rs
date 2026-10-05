//! Naming an agent or a tool: by its name, or once and for all.
//!
//! Every request that acts on an agent or a tool of the caller's — a
//! delete, an edit, a message, a logs read, a tag, an untag, an attach,
//! a detach — names it one of two ways, and the caller chooses which on
//! each request. By its NAME, as its create or its connect gave it, if
//! it gave one: the name names the agent or the tool now, and is free
//! again once that one is deleted. Or by its TEMPLATE and its INDEX,
//! the number its list item carries among all ever made from that
//! template: the pair names it once and for all, is given to nothing
//! else ever, and names it still after it is deleted — when a request
//! naming it that way finds nothing. A connected tool has no template,
//! and is named instead by the PROVIDER and container ID it joined,
//! which name it once and for all the same way. These are the variants
//! of [`Agent`] and of
//! [`Tool`], one JSON object each, told apart by their members:
//! `{"name":…}`, `{"template":…,"index":…}`, or for a connected tool
//! `{"provider":…,"id":…}`. An object carrying members of more than one
//! is malformed.
//!
//! A name is optional, so the once-and-for-all forms are the ones that
//! always work: an agent or a tool made with no name is reached by them
//! and no other way; a request naming a template and an index finds no
//! connected tool, and one naming a provider and an id finds no created
//! one.

mod agent;
mod tool;

pub use agent::*;
pub use tool::*;
