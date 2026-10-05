//! What a client can ask the daemon for.
//!
//! Each of these is one or more SCOPES — a request that opens one,
//! whatever channels either side needs inside it, and the answer that
//! comes back on channel `0`.
//!
//! | endpoint | scopes |
//! |----------|--------|
//! | [`agents`] | create an agent under a name; get one; delete one; send one a message; read one's log; list them, narrowed and run through a program; change what one mounts; tag one and untag one; make, get, list, delete, tag and untag the templates agents are made from |
//! | [`tools`] | create a tool under a name; get one; change what one mounts; hold somebody else's under a name; ask a provider which tool containers an identity runs; attach one to an agent; detach one; delete one; list them, narrowed and run through a program; tag one and untag one; set a dependency position's route to a tool, take it up, list the routes; make, get, list, delete, tag and untag the templates tools are made from |
//! | [`resources`] | upload a file or a directory, held by its hash; list them; delete one |
//! | [`providers`] | add a provider to dial, get one, list them, forget one, replace its mode; add a judge of providers that dial in, get one, list them, take one out, replace one |
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
//! | `19` | [`tools::list_for`] |
//! | `20` | [`tools::attach`] |
//! | `21` | [`tools::detach`] |
//! | `22` | [`tools::delete`] |
//! | `23` | [`tools::list`] |
//! | `24` | [`tools::tag`] |
//! | `25` | [`tools::untag`] |
//! | `26` | [`tools::routes::set`] |
//! | `27` | [`tools::routes::delete`] |
//! | `28` | [`tools::routes::list`] |
//! | `29` | [`tools::templates::create`] |
//! | `30` | [`tools::templates::get`] |
//! | `31` | [`tools::templates::list`] |
//! | `32` | [`tools::templates::delete`] |
//! | `33` | [`tools::templates::tag`] |
//! | `34` | [`tools::templates::untag`] |
//! | `35` | [`resources::upload`] |
//! | `36` | [`resources::list`] |
//! | `37` | [`resources::delete`] |
//! | `38` | [`providers::outgoing::add`] |
//! | `39` | [`providers::outgoing::get`] |
//! | `40` | [`providers::outgoing::list`] |
//! | `41` | [`providers::outgoing::delete`] |
//! | `42` | [`providers::outgoing::edit`] |
//! | `43` | [`providers::incoming::add`] |
//! | `44` | [`providers::incoming::get`] |
//! | `45` | [`providers::incoming::list`] |
//! | `46` | [`providers::incoming::delete`] |
//! | `47` | [`providers::incoming::edit`] |
//!
//! Forty-eight, so far. Tags are handed out in the order scopes are defined
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
pub mod providers;
pub mod resources;
pub mod tools;
