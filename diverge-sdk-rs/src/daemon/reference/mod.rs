//! Naming an agent or a tool: by its name, or once and for all.
//!
//! Every request that acts on an agent or a tool of the caller's —
//! a delete, an edit, a message, a logs read, a tag, an untag, an
//! attach, a detach — names it one of two ways, and the caller
//! chooses which on each request. By its NAME, as its create or its
//! connect gave it, if it gave one: the name names the agent or the
//! tool now, and is free again once that one is deleted. Or by its
//! TEMPLATE and its INDEX, the number its list item carries among all
//! ever made from that template: the pair names it once and for all,
//! is given to nothing else ever, and names it still after it is
//! deleted — when a request naming it that way finds nothing. A
//! connected tool has no template, and is named instead by the
//! PROVIDER and container ID it joined, which name it once and for
//! all the same way. These are the variants of [`Agent`] and of
//! [`Tool`], one JSON object each, told apart by their members:
//! `{"name":…}`, `{"template":…,"index":…}`, or for a connected tool
//! `{"provider":…,"id":…}` — or the string `"self"`. An object
//! carrying members of more than one is malformed.
//!
//! A fourth form names the caller itself: the string `"self"`. It
//! works one way only — [`Agent`]'s `"self"` when the caller is an
//! agent, and names that agent; [`Tool`]'s `"self"` when the caller
//! is a tool, and names that tool — through the daemon's own tools,
//! where the daemon knows who is calling from the scope the call
//! arrived on. A client has no self, an agent is no tool's self and a
//! tool no agent's: a request naming `"self"` where there is none
//! finds nothing. An agent or a tool that wants its own makers has
//! them already, in the `creator` chain of its own list item, so
//! there is no "parent": `"self"` and the chain are the whole
//! hierarchy.
//!
//! A name is optional, so the once-and-for-all forms are the ones
//! that always work: an agent or a tool made with no name is reached
//! by them and no other way; a request naming a template and an
//! index finds no connected tool, and one naming a provider and an id
//! finds no created one.

mod agent;
mod itself;
mod tool;

pub use agent::*;
pub use itself::*;
pub use tool::*;
