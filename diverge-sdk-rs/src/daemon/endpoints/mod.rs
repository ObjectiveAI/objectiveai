//! What a client can ask the daemon for.
//!
//! Each of these is one or more SCOPES — a request that opens one,
//! whatever channels either side needs inside it, and the answer that
//! comes back on channel `0`.
//!
//! | endpoint | scopes |
//! |----------|--------|
//! | [`agents`] | create an agent under a name; get one; delete one; send one a message; read one's log; list them, narrowed; change what one mounts; tag one and untag one; make, get, list, delete, tag and untag the templates agents are made from; download files out of one, upload files into one, transfer files out of one; watch one's container whole |
//! | [`tools`] | create a tool under a name; get one; change what one mounts; register another daemon's under a name; attach one to an agent; detach one; delete one; list them, narrowed; tag one and untag one; serve one to another daemon; make, get, list, delete, tag and untag the templates tools are made from; download files out of one, upload files into one, transfer files out of one; watch one's container whole |
//! | [`providers`] | add a provider to dial, get one, list them, forget one, replace its mode; add a credential of providers that dial in, get one, list them, take one out, replace one; add a daemon to connect to, get one, list them, forget one, replace its mode or its links, tag one and untag one |
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
//! | `18` | [`tools::register`] |
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
//! | `31` | [`providers::outgoing::add`] |
//! | `32` | [`providers::outgoing::get`] |
//! | `33` | [`providers::outgoing::list`] |
//! | `34` | [`providers::outgoing::delete`] |
//! | `35` | [`providers::outgoing::edit`] |
//! | `36` | [`providers::outgoing::tag`] |
//! | `37` | [`providers::outgoing::untag`] |
//! | `38` | [`providers::incoming::add`] |
//! | `39` | [`providers::incoming::get`] |
//! | `40` | [`providers::incoming::list`] |
//! | `41` | [`providers::incoming::delete`] |
//! | `42` | [`providers::incoming::edit`] |
//! | `43` | [`providers::incoming::tag`] |
//! | `44` | [`providers::incoming::untag`] |
//! | `45` | [`accounts::create`] |
//! | `46` | [`accounts::get`] |
//! | `47` | [`accounts::list`] |
//! | `48` | [`accounts::delete`] |
//! | `49` | [`accounts::edit`] |
//! | `50` | [`accounts::tag`] |
//! | `51` | [`accounts::untag`] |
//! | `52` | [`roles::create`] |
//! | `53` | [`roles::get`] |
//! | `54` | [`roles::list`] |
//! | `55` | [`roles::delete`] |
//! | `56` | [`roles::edit`] |
//! | `57` | [`roles::tag`] |
//! | `58` | [`roles::untag`] |
//! | `59` | [`agents::download`] |
//! | `60` | [`agents::upload`] |
//! | `61` | [`agents::transfer`] |
//! | `62` | [`tools::download`] |
//! | `63` | [`tools::upload`] |
//! | `64` | [`tools::transfer`] |
//! | `65` | [`volumes::create`] |
//! | `66` | [`volumes::get`] |
//! | `67` | [`volumes::list`] |
//! | `68` | [`volumes::delete`] |
//! | `69` | [`volumes::edit`] |
//! | `70` | [`volumes::tag`] |
//! | `71` | [`volumes::untag`] |
//! | `72` | [`volumes::stat`] |
//! | `73` | [`volumes::download`] |
//! | `74` | [`volumes::upload`] |
//! | `75` | [`volumes::transfer`] |
//! | `76` | [`agents::filetree`] |
//! | `77` | [`tools::filetree`] |
//! | `78` | [`volumes::filetree`] |
//! | `79` | [`postgres::get`] |
//! | `80` | [`postgres::list`] |
//! | `81` | [`tools::connect`] |
//! | `82` | [`providers::daemons::add`] |
//! | `83` | [`providers::daemons::get`] |
//! | `84` | [`providers::daemons::list`] |
//! | `85` | [`providers::daemons::delete`] |
//! | `86` | [`providers::daemons::edit`] |
//! | `87` | [`providers::daemons::tag`] |
//! | `88` | [`providers::daemons::untag`] |
//!
//! Eighty-nine, so far. Tags are handed out in the order scopes are defined
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
