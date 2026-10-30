//! What a client can ask the daemon for.
//!
//! Each of these is one or more SCOPES — a request that opens one,
//! whatever channels either side needs inside it, and the answer that
//! comes back on channel `0`.
//!
//! | endpoint | scopes |
//! |----------|--------|
//! | [`agents`] | create an agent under a name; get one; delete one; send one a message; list them, narrowed; change what one mounts; tag one and untag one; make, get, list, delete, tag and untag the templates agents are made from; download files out of one, upload files into one, transfer files out of one; watch one's container whole |
//! | [`standalone_tools`] | create a tool under a name; get one; change what one mounts; register another daemon's under a name; attach one to an agent, or to every agent of a template; detach one from either; delete one; list them, narrowed; tag one and untag one; serve one to another daemon; make, get, list, delete, tag and untag the templates tools are made from; download files out of one, upload files into one, transfer files out of one; watch one's container whole |
//! | [`dependency_tools`] | list the dependency tools agents' programs declare, one per template and owner, narrowed, each with the agents that depend on it and its run; get one; delete every tool of a template none of which runs; tag one and untag one; watch a running one's container whole; download files out of one, upload files into one, transfer files out of one |
//! | [`providers`] | add a provider to dial, get one, list them, forget one, replace its mode; add a credential of providers that dial in, get one, list them, take one out, replace one; add a daemon to connect to, get one, list them, forget one, replace its mode or its links, tag one and untag one |
//! | [`accounts`] | create an account — a name, a credential, or both — with its roles; get one; list them, narrowed; delete one; change one; tag one and untag one |
//! | [`roles`] | create a role, a named list of grants; get one; list them, narrowed; delete one; change one; tag one and untag one |
//! | [`vault`] | insert a value into a key — making the key, appending the value or replacing a slot's; get one key with every value and who holds each; list them by name, narrowed, never by value; delete one no agent or tool binds, or one free value of it; tag one and untag one |
//! | [`volumes`] | create a volume on a provider; get one; list them across providers, narrowed; delete one; change its size or mode; walk one for its use and its hash; download files out of one, upload files into one, transfer files out of one; see one's tree |
//! | [`postgres`] | see which database containers are served, the daemon's own or a remote one, as the daemon is configured; list the container connections open through it |
//! | [`events`] | list the events — who sent what to whom through the daemon, a message, an MCP call or an MCP read, and how often — narrowed, the latest so many, and keep the list |
//! | [`images`] | read an image's DIVERGE.md, from a container of it running or from a provider that can obtain it |
//! | [`info`] | say what the daemon is: its version |
//! | [`logs`] | read an agent's or a tool's log, narrowed, and watch it |
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
//! | `4` | unused |
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
//! | `15` | [`standalone_tools::create`] |
//! | `16` | [`standalone_tools::get`] |
//! | `17` | [`standalone_tools::edit`] |
//! | `18` | [`standalone_tools::register`] |
//! | `19` | [`standalone_tools::attach_agent_instance`] |
//! | `20` | [`standalone_tools::detach_agent_instance`] |
//! | `21` | [`standalone_tools::delete`] |
//! | `22` | [`standalone_tools::list`] |
//! | `23` | [`standalone_tools::tag`] |
//! | `24` | [`standalone_tools::untag`] |
//! | `25` | [`standalone_tools::templates::create`] |
//! | `26` | [`standalone_tools::templates::get`] |
//! | `27` | [`standalone_tools::templates::list`] |
//! | `28` | [`standalone_tools::templates::delete`] |
//! | `29` | [`standalone_tools::templates::tag`] |
//! | `30` | [`standalone_tools::templates::untag`] |
//! | `31` | unused |
//! | `32` | [`providers::get`] |
//! | `33` | [`providers::list`] |
//! | `34` | unused |
//! | `35` | unused |
//! | `36` | unused |
//! | `37` | unused |
//! | `38` | unused |
//! | `39` | unused |
//! | `40` | unused |
//! | `41` | unused |
//! | `42` | unused |
//! | `43` | unused |
//! | `44` | unused |
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
//! | `62` | [`standalone_tools::download`] |
//! | `63` | [`standalone_tools::upload`] |
//! | `64` | [`standalone_tools::transfer`] |
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
//! | `77` | [`standalone_tools::filetree`] |
//! | `78` | [`volumes::filetree`] |
//! | `79` | [`postgres::get`] |
//! | `80` | [`postgres::list`] |
//! | `81` | [`standalone_tools::connect`] |
//! | `82` | unused |
//! | `83` | [`providers::daemons::get`] |
//! | `84` | [`providers::daemons::list`] |
//! | `85` | unused |
//! | `86` | unused |
//! | `87` | unused |
//! | `88` | unused |
//! | `89` | [`dependency_tools::list`] |
//! | `90` | [`dependency_tools::get`] |
//! | `91` | [`dependency_tools::delete`] |
//! | `92` | [`dependency_tools::tag`] |
//! | `93` | [`dependency_tools::untag`] |
//! | `94` | [`dependency_tools::filetree`] |
//! | `95` | [`dependency_tools::download`] |
//! | `96` | [`dependency_tools::upload`] |
//! | `97` | [`dependency_tools::transfer`] |
//! | `98` | unused |
//! | `99` | unused |
//! | `100` | unused |
//! | `101` | [`standalone_tools::attach_agent_template`] |
//! | `102` | [`standalone_tools::detach_agent_template`] |
//! | `103` | [`vault::get`] |
//! | `104` | [`vault::insert`] |
//! | `105` | [`vault::delete`] |
//! | `106` | [`vault::list`] |
//! | `107` | [`vault::tag`] |
//! | `108` | [`vault::untag`] |
//! | `109` | [`events::list`] |
//! | `110` | [`images::info`] |
//! | `111` | [`info`] |
//! | `112` | unused |
//! | `113` | unused |
//! | `114` | [`logs`] |
//!
//! One hundred and fifteen, so far. Every one of them carries the optional
//! member `relay` — see [`Relay`](crate::shared::relay::Relay) — by
//! which a daemon serves it on another daemon, as an account it holds
//! there, as a conduit; the member is taken out before the request
//! goes on, so it is never relayed twice. Tags are handed out in the order scopes are defined
//! and nothing reads them in order; a new scope takes the next value
//! wherever it belongs conceptually. This table is the whole
//! allocation: each request states its own value and points here,
//! because a value chosen in one module has to be checked against every
//! other, and no module can see the others.
//!
//! [`ClientRequest`] is the same table as a type: one variant per row,
//! in tag order, plus an [`Invalid`](ClientRequest::Invalid) for a
//! payload that is none of them. It is the only place the values meet.
//! A request's NAME, for a client that takes one from a command line,
//! is its module path slashed — `agents/create`,
//! `standalone_tools/templates/list` — and [`tag`] is the name to the
//! tag, [`ClientRequest::path`] the tag to the name.
//!
//! # The answers as JSON
//!
//! Every response frame serializes to JSON, for a client that prints
//! what the daemon answered: the variant, snake case, over its payload
//! — `{"found":…}` — and a bare variant the string alone —
//! `"not_found"`. The wire never carries this form; a download's
//! chunk writes its path and its size and not its body.
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
mod refusal;

pub use client_request::*;

pub mod accounts;
pub mod agents;
pub mod dependency_tools;
pub mod events;
pub mod images;
pub mod info;
pub mod logs;
pub mod postgres;
pub mod providers;
pub mod roles;
pub mod standalone_tools;
pub mod vault;
pub mod volumes;
