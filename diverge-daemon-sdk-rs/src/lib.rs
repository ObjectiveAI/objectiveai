//! The SDK of the Diverge daemon.
//!
//! The daemon speaks the same frames, scopes and channels the provider
//! protocol does, so this crate takes the wire from
//! [`diverge_provider_sdk`] — its [`frame`](diverge_provider_sdk::frame),
//! [`connection`](diverge_provider_sdk::connection),
//! [`encode`](diverge_provider_sdk::encode) and
//! [`decode`](diverge_provider_sdk::decode), the one error shape in
//! [`shared::error`](diverge_provider_sdk::shared::error), and under
//! the `client` and `server` features the frame-level cores of each
//! half — and defines its own [`endpoints`] on top, as the provider
//! SDK defines its own. The provider's endpoints are not the daemon's,
//! and nothing here names them but the container request a daemon
//! spawns an agent from.

pub mod endpoints;
