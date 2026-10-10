//! What a client can ask the daemon for.
//!
//! Each of these is one or more SCOPES — a request that opens one,
//! whatever channels either side needs inside it, and the answer that
//! comes back on channel `0`.
//!
//! | endpoint | scopes |
//! |----------|--------|
//! | [`agents`] | create an agent under a name; get one; delete one; send one a message; read one's log; list them, narrowed; change what one mounts; tag one and untag one; make, get, list, delete, tag and untag the templates agents are made from; download files out of one, upload files into one, transfer files out of one; watch one's container whole |
//! | [`tools`] | create a tool under a name; get one; change what one mounts; hold somebody else's under a name; ask a provider which tool containers an identity runs; attach one to an agent; detach one; delete one; list them, narrowed; tag one and untag one; admit a lister or a connector to one, take the admission back; make, get, list, delete, tag and untag the templates tools are made from; download files out of one, upload files into one, transfer files out of one; watch one's container whole |
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
//! | `26` | [`tools::templates::create`] |
//! | `27` | [`tools::templates::get`] |
//! | `28` | [`tools::templates::list`] |
//! | `29` | [`tools::templates::delete`] |
//! | `30` | [`tools::templates::tag`] |
//! | `31` | [`tools::templates::untag`] |
//! | `32` | [`providers::outgoing::add`] |
//! | `33` | [`providers::outgoing::get`] |
//! | `34` | [`providers::outgoing::list`] |
//! | `35` | [`providers::outgoing::delete`] |
//! | `36` | [`providers::outgoing::edit`] |
//! | `37` | [`providers::outgoing::tag`] |
//! | `38` | [`providers::outgoing::untag`] |
//! | `39` | [`providers::incoming::add`] |
//! | `40` | [`providers::incoming::get`] |
//! | `41` | [`providers::incoming::list`] |
//! | `42` | [`providers::incoming::delete`] |
//! | `43` | [`providers::incoming::edit`] |
//! | `44` | [`providers::incoming::tag`] |
//! | `45` | [`providers::incoming::untag`] |
//! | `46` | [`accounts::create`] |
//! | `47` | [`accounts::get`] |
//! | `48` | [`accounts::list`] |
//! | `49` | [`accounts::delete`] |
//! | `50` | [`accounts::edit`] |
//! | `51` | [`accounts::tag`] |
//! | `52` | [`accounts::untag`] |
//! | `53` | [`roles::create`] |
//! | `54` | [`roles::get`] |
//! | `55` | [`roles::list`] |
//! | `56` | [`roles::delete`] |
//! | `57` | [`roles::edit`] |
//! | `58` | [`roles::tag`] |
//! | `59` | [`roles::untag`] |
//! | `60` | [`agents::download`] |
//! | `61` | [`agents::upload`] |
//! | `62` | [`agents::transfer`] |
//! | `63` | [`tools::download`] |
//! | `64` | [`tools::upload`] |
//! | `65` | [`tools::transfer`] |
//! | `66` | [`volumes::create`] |
//! | `67` | [`volumes::get`] |
//! | `68` | [`volumes::list`] |
//! | `69` | [`volumes::delete`] |
//! | `70` | [`volumes::edit`] |
//! | `71` | [`volumes::tag`] |
//! | `72` | [`volumes::untag`] |
//! | `73` | [`volumes::stat`] |
//! | `74` | [`volumes::download`] |
//! | `75` | [`volumes::upload`] |
//! | `76` | [`volumes::transfer`] |
//! | `77` | [`agents::filetree`] |
//! | `78` | [`tools::filetree`] |
//! | `79` | [`volumes::filetree`] |
//! | `80` | [`postgres::get`] |
//! | `81` | [`postgres::list`] |
//! | `82` | [`tools::admit`] |
//! | `83` | [`tools::unadmit`] |
//!
//! Eighty-four, so far. Tags are handed out in the order scopes are defined
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
