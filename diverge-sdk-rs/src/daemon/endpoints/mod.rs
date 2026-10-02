//! What a client can ask the daemon for.
//!
//! Each of these is one or more SCOPES — a request that opens one,
//! whatever channels either side needs inside it, and the answer that
//! comes back on channel `0`.
//!
//! | endpoint | scopes |
//! |----------|--------|
//! | [`agents`] | create an agent under a name; get one; delete one; send one a message; read one's log; list them, narrowed and run through a program; change what one mounts; tag one and untag one; make, get, list, delete, tag and untag the templates agents are made from |
//! | [`tools`] | create a tool under a name; get one; change what one mounts; hold somebody else's under a name; attach one to an agent; detach one; delete one; list them, narrowed and run through a program; tag one and untag one; make, get, list, delete, tag and untag the templates tools are made from |
//! | [`resources`] | upload a file or a directory, held by its hash; list them; delete one |
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
//! | `0` | [`agents::create`] |
//! | `1` | [`agents::get`] |
//! | `2` | [`agents::delete`] |
//! | `3` | [`agents::message`] |
//! | `4` | [`agents::logs`] |
//! | `5` | [`agents::list`] |
//! | `6` | [`agents::edit`] |
//! | `7` | [`agents::tag`] |
//! | `8` | [`agents::untag`] |
//! | `9` | [`agents::templates::create`] |
//! | `10` | [`agents::templates::get`] |
//! | `11` | [`agents::templates::list`] |
//! | `12` | [`agents::templates::delete`] |
//! | `13` | [`agents::templates::tag`] |
//! | `14` | [`agents::templates::untag`] |
//! | `15` | [`tools::create`] |
//! | `16` | [`tools::get`] |
//! | `17` | [`tools::edit`] |
//! | `18` | [`tools::connect`] |
//! | `19` | [`tools::attach`] |
//! | `20` | [`tools::detach`] |
//! | `21` | [`tools::delete`] |
//! | `22` | [`tools::list`] |
//! | `23` | [`tools::tag`] |
//! | `24` | [`tools::untag`] |
//! | `25` | [`tools::templates::create`] |
//! | `26` | [`tools::templates::get`] |
//! | `27` | [`tools::templates::list`] |
//! | `28` | [`tools::templates::delete`] |
//! | `29` | [`tools::templates::tag`] |
//! | `30` | [`tools::templates::untag`] |
//! | `31` | [`resources::upload`] |
//! | `32` | [`resources::list`] |
//! | `33` | [`resources::delete`] |
//!
//! Thirty-four, so far. Tags are handed out in the order scopes are defined
//! and nothing reads them in order; a new scope takes the next value
//! wherever it belongs conceptually. This table is the whole
//! allocation: each request states its own value and points here,
//! because a value chosen in one module has to be checked against
//! every other, and no module can see the others.
//!
//! [`ClientRequest`] is the same table as a type: one variant per row,
//! in tag order, plus an [`Invalid`](ClientRequest::Invalid) for a
//! payload that is none of them. It is the only place the values meet.
//!
//! # The wire is the provider's
//!
//! The header, the seven frame types, scopes and channels, and the
//! encode and decode contract are
//! [`provider`](crate::provider)'s, taken as they are; the daemon's tag
//! table is its own, and a tag here means nothing on the provider's
//! wire, nor the other way round.

mod client_request;

pub use client_request::*;

pub mod agents;
pub mod resources;
pub mod tools;
