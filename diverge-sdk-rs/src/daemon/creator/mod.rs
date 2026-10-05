//! Who made a thing: the client, an agent, or a tool.
//!
//! Everything the daemon holds for a caller — an agent, a tool, a
//! template of either kind, a route — was made by somebody: by the
//! client itself, over an endpoint, or by an agent or a tool of the
//! client's, through the daemon. Every list item carries
//! that one maker as its `creator`, a [`Creator`]; a user part of a
//! message carries the one that sent it as its `sender`, the same
//! shape. Only the direct maker is carried: the maker's own maker is
//! on the maker's list item, and the lineage is read one item at a
//! time, as far back as it goes, to the client.
//!
//! A creator is named once and for all, so that what it made still
//! says so after the creator is deleted and after its name is given
//! to another: an [`Agent`] by its template and its index among the
//! agents ever made from that template, a [`Tool`] by its template
//! and its index among the tools ever made from that template, the
//! [`Client`] by its identity. The name is carried beside, when the
//! create gave one. A connected tool is somebody else's container and
//! calls nothing of the client's, so it makes nothing and is never a
//! creator; a tool here is always one the client made.
//!
//! Every list narrows by creator: a request naming creators matches
//! what was made by any one of them, directly.

mod agent;
mod client;
mod creator;
mod tool;

pub use agent::*;
pub use client::*;
pub use creator::*;
pub use tool::*;
