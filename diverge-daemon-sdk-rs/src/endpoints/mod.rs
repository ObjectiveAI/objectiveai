//! What a client can ask the daemon for.
//!
//! Each of these is one or more SCOPES — a request that opens one,
//! whatever channels either side needs inside it, and the answer that
//! comes back on channel `0`.
//!
//! | endpoint | scopes |
//! |----------|--------|
//! | [`agents`] | create an agent under a name; delete one by name; send one a message; read one's log; list them all |
//! | [`volumes`] | the daemon's own volumes, as a provider's: list them; examine, read, write, walk, resize or destroy one; ask how large one may be made or grow; make one |
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
//! | `1` | [`agents::delete`] |
//! | `2` | [`agents::message`] |
//! | `3` | [`agents::logs`] |
//! | `4` | [`agents::list`] |
//! | `5` | [`volumes::list`] |
//! | `6` | [`volumes::stat`] |
//! | `7` | [`volumes::read`] |
//! | `8` | [`volumes::write`] |
//! | `9` | [`volumes::filetree`] |
//! | `10` | [`volumes::create_capacity`] |
//! | `11` | [`volumes::create`] |
//! | `12` | [`volumes::edit_capacity`] |
//! | `13` | [`volumes::edit`] |
//! | `14` | [`volumes::delete`] |
//!
//! Fifteen, so far. The ten volume scopes are the provider's ten, in
//! the provider's order, behind the daemon's own tags. Tags are handed out in the order scopes are defined
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
//! [`diverge_provider_sdk`]'s, taken as they are; the daemon's tag
//! table is its own, and a tag here means nothing on the provider's
//! wire, nor the other way round.

mod client_request;

pub use client_request::*;

pub mod agents;
pub mod volumes;
