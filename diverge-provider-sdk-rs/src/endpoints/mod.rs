//! What a client can ask a provider for.
//!
//! Each of these is one or more SCOPES — a request that opens one,
//! whatever channels either side needs inside it, and the answer that
//! comes back on channel `0`.
//!
//! | endpoint | scopes |
//! |----------|--------|
//! | [`containers`] | run an agent or a tool server in a container; join a tool server |
//! | [`images`] | ask whether an image can be supplied |
//! | [`volumes`] | list what a provider offers; examine one; ask how large one may be made; make one; ask how far one may grow; resize it; destroy it |
//! | [`version`] | ask what a provider is |
//!
//! # The tags
//!
//! Every request names itself with one byte at the front of its
//! payload. The frame layer never reads it — it carries one kind of
//! request frame and hands the bytes on — so this is the only thing
//! telling one request from another.
//!
//! | tag | request |
//! |-----|---------|
//! | `0` | [`containers::agents::run`] |
//! | `1` | [`containers::tools::run`] |
//! | `2` | [`containers::tools::connect`] |
//! | `3` | [`volumes::list`] |
//! | `4` | [`volumes::stat`] |
//! | `5` | [`volumes::create_capacity`] |
//! | `6` | [`volumes::create`] |
//! | `7` | [`volumes::edit_capacity`] |
//! | `8` | [`volumes::edit`] |
//! | `9` | [`volumes::delete`] |
//! | `10` | [`images::check`] |
//! | `11` | [`version`] |
//!
//! Twelve, grouped by endpoint and ordered within it. The three
//! container scopes lead: the agents' run, then the tools' run and the
//! connect that joins one. The seven volume scopes follow in the order
//! a caller uses them: find one, examine it, ask how large one may be
//! made, make one, ask how far one may grow, resize it, destroy it.
//! Then the two that ask rather than do:
//! [`images::check`], and [`version`].
//!
//! Nothing derives meaning from adjacency, which [`version`] is the
//! proof of — it is the one a client asks FIRST and it holds the
//! highest tag, because tags are handed out in the order scopes were
//! defined and nothing reads them in order. The grouping is for
//! whoever reads the table, and a new scope takes `12` wherever it
//! belongs conceptually.
//!
//! This table is the whole allocation. Each request states its own
//! value and points here, because a value chosen in one module has to
//! be checked against every other, and no module can see the others.
//!
//! [`ClientRequest`] is the same table as a type: one variant per row,
//! in tag order, plus an
//! [`Invalid`](ClientRequest::Invalid) for a payload that is none of
//! them. It is the only place the values meet.
//!
//! # One substrate, two families
//!
//! [`containers`] is the substrate, and the two families under it are
//! told apart by what a caller says into the container: a message
//! for an agent, or MCP. Everything else a container scope carries —
//! the arguments a container is made with and the schema that says
//! what they may be among it — is the same wire in all three, defined
//! once in [`shared::containers`](crate::shared::containers).

mod client_request;

pub use client_request::*;

pub mod containers;
pub mod images;
pub mod version;
pub mod volumes;
