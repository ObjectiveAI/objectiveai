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
//! | [`volumes`] | list the directories a provider offers; watch one |
//! | [`laboratories`] | create a laboratory; join one |
//! | [`mcp_plugin`] | run a plugin, call it |
//!
//! Seven requests in total, and each names itself with one tag value
//! at the front of its payload — `0` through `6`. The frame layer
//! never reads them; it carries one kind of request frame and hands
//! the bytes on.
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

pub mod agentic_loop;
pub mod images;
pub mod laboratories;
pub mod mcp_plugin;
pub mod volumes;
