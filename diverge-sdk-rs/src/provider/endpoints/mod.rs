//! What a client can ask a provider for.
//!
//! Each of these is one or more SCOPES — a request that opens one,
//! whatever channels either side needs inside it, and the answer that
//! comes back on channel `0`.
//!
//! | endpoint | scopes |
//! |----------|--------|
//! | [`containers`] | run an agent or a tool server in a container; join a tool server; find the tool containers an identity runs |
//! | [`images`] | ask whether an image can be supplied |
//! | [`volumes`] | list what a provider offers; examine one; read, write or walk one; serve one's files live and watch them as served; ask how large one may be made; make one; ask how far one may grow; resize it; destroy it |
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
//! | `3` | [`containers::tools::list_for`] |
//! | `4` | [`volumes::list`] |
//! | `5` | [`volumes::stat`] |
//! | `6` | [`volumes::read`] |
//! | `7` | [`volumes::write`] |
//! | `8` | [`volumes::filetree`] |
//! | `9` | [`volumes::serve`] |
//! | `10` | [`volumes::create_capacity`] |
//! | `11` | [`volumes::create`] |
//! | `12` | [`volumes::edit_capacity`] |
//! | `13` | [`volumes::edit`] |
//! | `14` | [`volumes::delete`] |
//! | `15` | [`images::check`] |
//! | `16` | [`version`] |
//!
//! Seventeen, grouped by endpoint and ordered within it. The four
//! container scopes lead: the agents' run, the tools' run, the connect
//! that joins one, and the list that finds somebody else's. The eleven volume
//! scopes follow in the
//! order a caller uses them: find one, examine it, read a file out of
//! it, write one in, see its tree, serve its files live, ask how large
//! one may be made, make one, ask how far one may grow, resize it,
//! destroy it.
//! Then the two that ask rather than do:
//! [`images::check`], and [`version`].
//!
//! Nothing derives meaning from adjacency, which [`version`] is the
//! proof of — it is the one a client asks FIRST and it holds the
//! highest tag, because tags are handed out in the order scopes were
//! defined and nothing reads them in order. The grouping is for
//! whoever reads the table, and a new scope takes the next free value wherever it
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
