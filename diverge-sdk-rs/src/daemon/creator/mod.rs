//! Who made a thing: the client, an agent, or a tool, and through
//! whom.
//!
//! Everything the daemon holds for a caller — an agent, a tool, a
//! template of either kind — was made by somebody: by the client
//! itself, over an endpoint, or by an agent or a tool of the
//! client's, which was itself made by somebody. The daemon keeps
//! the whole chain, and every list item carries it as its
//! `creator`: a list of [`Creator`], never empty, whose first is
//! always the client and whose last is what made the item itself,
//! each one between made by the one before it. A thing the client
//! made directly has a chain of one.
//!
//! A creator is named once and for all, so that the chain stays true
//! after the creator is deleted and after its name is given to
//! another: an [`Agent`] by its template and its index among the
//! agents ever made from that template, a [`Tool`] by its template
//! and its index among the tools ever made from that template, the
//! [`Client`] by its identity. The name is carried beside, as the
//! create gave it. A connected tool is somebody else's container and
//! calls nothing of the client's, so it makes nothing and is never
//! a creator; a tool here is always one the client made.
//!
//! Every list narrows by creator: a request naming creators matches
//! what was made under any one of them, anywhere in its chain.

mod agent;
mod client;
mod creator;
mod tool;

pub use agent::*;
pub use client::*;
pub use creator::*;
pub use tool::*;
