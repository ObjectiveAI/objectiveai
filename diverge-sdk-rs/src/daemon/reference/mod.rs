//! Naming an agent or a tool: by its name, or once and for all.
//!
//! Every request that acts on an agent or a tool of the caller's —
//! a delete, an edit, a message, a logs read, a tag, an untag, an
//! attach, a detach — names it one of two ways, and the caller
//! chooses which on each request. By its NAME, as its create or its
//! connect gave it: the name names the agent or the tool now, and is
//! free again once that one is deleted. Or by its TEMPLATE and its
//! COUNT, the number its list item carries among all ever made from
//! that template: the pair names it once and for all, is given to
//! nothing else ever, and names it still after it is deleted — when
//! a request naming it that way finds nothing. The two are the
//! two variants of [`Agent`] and of [`Tool`], one JSON object each,
//! told apart by their members: `{"name":…}`, or
//! `{"template":…,"count":…}`. An object carrying members of both
//! is malformed.
//!
//! A connected tool has no template, so it is named by its name
//! alone; a request naming a template and a count finds no
//! connected tool.

mod agent;
mod tool;

pub use agent::*;
pub use tool::*;
