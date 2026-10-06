//! What a client can ask the daemon for.
//!
//! Each of these is one or more SCOPES — a request that opens one,
//! whatever channels either side needs inside it, and the answer that
//! comes back on channel `0`.
//!
//! | endpoint | scopes |
//! |----------|--------|
//! | [`agents`] | create an agent under a name; get one; delete one; send one a message; read one's log; list them, narrowed and run through a program; change what one mounts; tag one and untag one; make, get, list, delete, tag and untag the templates agents are made from; download files out of one, upload files into one, transfer files out of one; watch one's container whole |
//! | [`tools`] | create a tool under a name; get one; change what one mounts; hold somebody else's under a name; ask a provider which tool containers an identity runs; attach one to an agent; detach one; delete one; list them, narrowed and run through a program; tag one and untag one; set a dependency position's route to a tool, take it up, list the routes; make, get, list, delete, tag and untag the templates tools are made from; download files out of one, upload files into one, transfer files out of one; watch one's container whole |
//! | [`resources`] | upload a file or a directory, held by its hash; get one; list them, narrowed; delete one; tag one and untag one; download one, transfer one into a container; see one's tree |
//! | [`providers`] | add a provider to dial, get one, list them, forget one, replace its mode; add a credential of providers that dial in, get one, list them, take one out, replace one |
//! | [`accounts`] | create an account — a name, a credential, or both — with its roles; get one; list them, narrowed; delete one; change one; tag one and untag one |
//! | [`roles`] | create a role, a named list of grants; get one; list them, narrowed; delete one; change one; tag one and untag one |
//! | [`volumes`] | create a volume on a provider; get one; list them across providers, narrowed; delete one; change its size or mode; walk one for its use and its hash; download files out of one, upload files into one, transfer files out of one; see one's tree |
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
//! | `48` | [`accounts::create`] |
//! | `49` | [`accounts::get`] |
//! | `50` | [`accounts::list`] |
//! | `51` | [`accounts::delete`] |
//! | `52` | [`accounts::edit`] |
//! | `53` | [`accounts::tag`] |
//! | `54` | [`accounts::untag`] |
//! | `55` | [`roles::create`] |
//! | `56` | [`roles::get`] |
//! | `57` | [`roles::list`] |
//! | `58` | [`roles::delete`] |
//! | `59` | [`roles::edit`] |
//! | `60` | [`roles::tag`] |
//! | `61` | [`roles::untag`] |
//! | `62` | [`agents::download`] |
//! | `63` | [`agents::upload`] |
//! | `64` | [`agents::transfer`] |
//! | `65` | [`tools::download`] |
//! | `66` | [`tools::upload`] |
//! | `67` | [`tools::transfer`] |
//! | `68` | [`resources::download`] |
//! | `69` | [`resources::transfer`] |
//! | `70` | [`volumes::create`] |
//! | `71` | [`volumes::get`] |
//! | `72` | [`volumes::list`] |
//! | `73` | [`volumes::delete`] |
//! | `74` | [`volumes::edit`] |
//! | `75` | [`volumes::stat`] |
//! | `76` | [`volumes::download`] |
//! | `77` | [`volumes::upload`] |
//! | `78` | [`volumes::transfer`] |
//! | `79` | [`agents::filetree`] |
//! | `80` | [`tools::filetree`] |
//! | `81` | [`resources::filetree`] |
//! | `82` | [`volumes::filetree`] |
//! | `83` | [`resources::get`] |
//! | `84` | [`resources::tag`] |
//! | `85` | [`resources::untag`] |
//!
//! Eighty-six, so far. Tags are handed out in the order scopes are
//! defined and nothing reads them in order; a new scope takes the next
//! value wherever it belongs conceptually. This table is the whole
//! allocation: each request states its own value and points here,
//! because a value chosen in one module has to be checked against every
//! other, and no module can see the others.
//!
//! [`ClientRequest`] is the same table as a type: one variant per row,
//! in tag order, plus an [`Invalid`](ClientRequest::Invalid) for a
//! payload that is none of them. It is the only place the values meet.
//!
//! # Who asks, and what they may
//!
//! Every request is served for one [account](accounts) — the account
//! the connection dialed in as, or the `account` of the container the
//! request came from — and is allowed or refused by the
//! [grants](crate::daemon::grant) of the [`roles`] that account
//! holds. Every response has a `Forbidden` answer for the refusal,
//! tagged just before its error.
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

pub mod accounts;
pub mod agents;
pub mod providers;
pub mod resources;
pub mod roles;
pub mod tools;
pub mod volumes;
