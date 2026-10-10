//! What a client can ask the daemon for.
//!
//! Each of these is one or more SCOPES — a request that opens one,
//! whatever channels either side needs inside it, and the answer that
//! comes back on channel `0`.
//!
//! | endpoint | scopes |
//! |----------|--------|
//! | [`agents`] | create an agent under a name; get one; delete one; send one a message; read one's log; list them, narrowed; change what one mounts; tag one and untag one; make, get, list, delete, tag and untag the templates agents are made from; download files out of one, upload files into one, transfer files out of one; watch one's container whole |
//! | [`tools`] | create a tool under a name; get one; change what one mounts; hold somebody else's under a name; ask a provider which tool containers an identity runs; attach one to an agent; detach one; delete one; list them, narrowed; tag one and untag one; admit a lister or a connector to one, take the admission back; set a dependency position's route to a tool, take it up, list the routes; make, get, list, delete, tag and untag the templates tools are made from; download files out of one, upload files into one, transfer files out of one; watch one's container whole |
//! | [`providers`] | add a provider to dial, get one, list them, forget one, replace its mode; add a credential of providers that dial in, get one, list them, take one out, replace one |
//! | [`accounts`] | create an account — a name, a credential, or both — with its roles; get one; list them, narrowed; delete one; change one; tag one and untag one |
//! | [`roles`] | create a role, a named list of grants; get one; list them, narrowed; delete one; change one; tag one and untag one |
//! | [`volumes`] | create a volume on a provider; get one; list them across providers, narrowed; delete one; change its size or mode; walk one for its use and its hash; download files out of one, upload files into one, transfer files out of one; see one's tree |
//! | [`postgres`] | see which database containers are served, the daemon's own or a remote one, as the daemon is configured; list the container connections open through it |
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
//! | `35` | [`providers::outgoing::add`] |
//! | `36` | [`providers::outgoing::get`] |
//! | `37` | [`providers::outgoing::list`] |
//! | `38` | [`providers::outgoing::delete`] |
//! | `39` | [`providers::outgoing::edit`] |
//! | `40` | [`providers::outgoing::tag`] |
//! | `41` | [`providers::outgoing::untag`] |
//! | `42` | [`providers::incoming::add`] |
//! | `43` | [`providers::incoming::get`] |
//! | `44` | [`providers::incoming::list`] |
//! | `45` | [`providers::incoming::delete`] |
//! | `46` | [`providers::incoming::edit`] |
//! | `47` | [`providers::incoming::tag`] |
//! | `48` | [`providers::incoming::untag`] |
//! | `49` | [`accounts::create`] |
//! | `50` | [`accounts::get`] |
//! | `51` | [`accounts::list`] |
//! | `52` | [`accounts::delete`] |
//! | `53` | [`accounts::edit`] |
//! | `54` | [`accounts::tag`] |
//! | `55` | [`accounts::untag`] |
//! | `56` | [`roles::create`] |
//! | `57` | [`roles::get`] |
//! | `58` | [`roles::list`] |
//! | `59` | [`roles::delete`] |
//! | `60` | [`roles::edit`] |
//! | `61` | [`roles::tag`] |
//! | `62` | [`roles::untag`] |
//! | `63` | [`agents::download`] |
//! | `64` | [`agents::upload`] |
//! | `65` | [`agents::transfer`] |
//! | `66` | [`tools::download`] |
//! | `67` | [`tools::upload`] |
//! | `68` | [`tools::transfer`] |
//! | `69` | [`volumes::create`] |
//! | `70` | [`volumes::get`] |
//! | `71` | [`volumes::list`] |
//! | `72` | [`volumes::delete`] |
//! | `73` | [`volumes::edit`] |
//! | `74` | [`volumes::tag`] |
//! | `75` | [`volumes::untag`] |
//! | `76` | [`volumes::stat`] |
//! | `77` | [`volumes::download`] |
//! | `78` | [`volumes::upload`] |
//! | `79` | [`volumes::transfer`] |
//! | `80` | [`agents::filetree`] |
//! | `81` | [`tools::filetree`] |
//! | `82` | [`volumes::filetree`] |
//! | `83` | [`postgres::get`] |
//! | `84` | [`postgres::list`] |
//! | `85` | [`tools::admit`] |
//! | `86` | [`tools::unadmit`] |
//!
//! Eighty-seven, so far. Tags are handed out in the order scopes are defined
//! and nothing reads them in order; a new scope takes the next value
//! wherever it belongs conceptually. This table is the whole
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
//! [grants](crate::daemon::grant) of the [`roles`] that account holds.
//! Every response has a `Forbidden` answer for the refusal, tagged just
//! before its error.
//!
//! # The wire is the provider's
//!
//! The header, the seven frame types, scopes and channels, and the
//! encode and decode contract are [`provider`](crate::provider)'s,
//! taken as they are; the daemon's tag table is its own, and a tag here
//! means nothing on the provider's wire, nor the other way round.

mod client_request;

pub use client_request::*;

pub mod accounts;
pub mod agents;
pub mod postgres;
pub mod providers;
pub mod roles;
pub mod tools;
pub mod volumes;
