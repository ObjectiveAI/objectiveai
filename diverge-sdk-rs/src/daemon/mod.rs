//! The daemon protocol: what a caller asks the Diverge daemon for.
//!
//! The daemon speaks the same frames, scopes and channels the provider
//! protocol does, over the same [`wire`](crate::wire) — its
//! [`frame`](crate::wire::frame), [`connection`](crate::wire::connection),
//! [`encode`](crate::wire::encode) and [`decode`](crate::wire::decode),
//! the one error shape in [`shared::error`](crate::shared::error), and
//! the frame-level cores of each half — and defines its own
//! [`endpoints`] on top, as the [`provider`](crate::provider) defines
//! its own. The provider's endpoints are not the daemon's, and nothing
//! here names them but what a daemon spawns an agent from.

pub mod endpoints;
