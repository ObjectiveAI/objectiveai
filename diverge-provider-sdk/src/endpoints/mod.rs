//! What a client can ask a provider for.
//!
//! Each of these is one or more SCOPES — a request that opens one,
//! whatever channels either side needs inside it, and the answer that
//! comes back on channel `0`.
//!
//! | endpoint | scopes |
//! |----------|--------|
//! | [`agentic_loop`] | run an agent, stream what it does |
//! | [`images`] | ask whether an image can be supplied |
//! | [`volumes`] | list what a provider offers; watch one; make, resize or destroy one |
//! | [`laboratories`] | run a laboratory; join one |
//! | [`mcp_plugin`] | run a plugin, call it |
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
//! | `0` | [`agentic_loop::run`] |
//! | `1` | [`images::check`] |
//! | `2` | [`volumes::list`] |
//! | `3` | [`volumes::watch`] |
//! | `4` | [`laboratories::run`] |
//! | `5` | [`laboratories::connect`] |
//! | `6` | [`mcp_plugin::run`] |
//! | `7` | [`volumes::create`] |
//! | `8` | [`volumes::delete`] |
//! | `9` | [`volumes::edit`] |
//!
//! Ten, and they are in the order they were allocated rather than
//! grouped by endpoint — [`volumes`] holds `2`, `3`, `7`, `8` and `9`.
//! Nothing derives meaning from adjacency, so regrouping them would
//! change every implementation to make a table look tidier.
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
//! # Named for what runs in them
//!
//! Three of these are containers, and the three are told apart by what
//! runs inside. An [`agentic_loop`] runs an agent in one, a
//! [`laboratory`](laboratories) is one an agent works inside, and an
//! [`mcp_plugin`] is one that serves tools. So `containers` was never
//! a distinction — it was the substrate, and a module named after it
//! would have grouped things by the one property they all share.
//!
//! What they are NOT all built from is in
//! [`shared`](crate::shared) — including
//! [`container`](crate::shared::container), which is the reading,
//! writing and moving of files that any of them can be asked to do.

mod client_request;

pub use client_request::*;

pub mod agentic_loop;
pub mod images;
pub mod laboratories;
pub mod mcp_plugin;
pub mod volumes;
