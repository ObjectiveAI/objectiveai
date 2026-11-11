//! Editing a connected tool: a patch to its name.
//!
//! One request, one answer. A client names a connected tool of its own
//! and a patch — see [`patch`](crate::daemon::patch) — to the tool's
//! editable members, its name alone; the daemon applies it, whole or
//! not at all, and answers that the tool is as the result states, that
//! no connected tool is the one named, that the name is another's, or
//! that it failed, and the scope finishes. The daemon, the route and
//! the tool it is held from are fixed at its
//! [`create`](super::create): a tool reached another way is another
//! create.
//!
//! Split by who SENDS, as everywhere else. A client asks — so the
//! question is in [`client`] — and the daemon answers, so the answer
//! is in [`server`]. Neither side holds both halves of the exchange.

pub mod client;
pub mod server;
