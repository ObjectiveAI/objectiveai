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
//! | [`filesystem`] | list watchable directories; watch one |
//! | [`containers`] | create a container; join one |
//!
//! Six requests in total, and each names itself with one tag value at
//! the front of its payload — `0` through `5`. The frame layer never
//! reads them; it carries one kind of request frame and hands the
//! bytes on.
//!
//! What these are BUILT from, where more than one of them needs the
//! same thing, is in [`shared`](crate::shared).

pub mod agentic_loop;
pub mod containers;
pub mod filesystem;
pub mod images;
