//! The seam between this app and the Diverge daemon.
//!
//! One trait, one method per verb `diverge_sdk::daemon` defines, taking and
//! returning that module's own types. `stub::StubDaemon` (feature
//! `stand-in`) stands in until Ronald ships a daemon; a `WireDaemon` over
//! the SDK's caller half is the whole of meeting up with him. Nothing above
//! this module knows which one it holds.
//!
//! Since `87015ef92` the daemon has no volumes: they are each machine's,
//! behind the other seam, [`crate::machines`].
//!
//! Three methods are **ours until the wire has them** — `providers_*`. The
//! app adds machines inside itself (Maya, 2026-09-25); today a peer is a
//! `config.yaml` entry and the daemon has no verb for it.

use std::pin::Pin;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use futures::Stream;
use tokio_util::sync::CancellationToken;

use diverge_sdk::daemon::endpoints::{agents, tools};
use diverge_sdk::daemon::endpoints::agents::logs::server::response::Identity;

pub mod jq;
#[cfg(feature = "stand-in")]
pub mod stub;

/// What a scope sends, in order, until it finishes.
pub type Frames<T> = Pin<Box<dyn Stream<Item = T> + Send + 'static>>;

/// A machine the daemon can run agents on. Ours until the wire has it.
#[derive(Debug, Clone)]
pub struct ProviderEntry {
    /// As the daemon names it — never anything the app decided.
    pub identity: Identity,
    pub added: DateTime<Utc>,
}

/// The two ways the daemon comes to know a provider, mirroring
/// [`Identity`]'s two kinds. The key goes in and never comes back out.
#[derive(Debug, Clone)]
pub enum NewProvider {
    /// "I dial them": the daemon dials `address`, presenting `key`.
    Dial { address: String, key: String },
    /// "They dial me": a provider presenting `key` is known as `identity`.
    Accept { identity: String, key: String },
}

#[async_trait]
pub trait Daemon: Send + Sync + 'static {
    /// Tag 0. Spawn an agent under a name.
    async fn agents_create(
        &self,
        request: agents::create::client::request::Frame,
    ) -> agents::create::server::response::Frame;

    /// Tag 1. Delete an agent by name. An active agent is left as it is.
    async fn agents_delete(
        &self,
        request: agents::delete::client::request::Frame,
    ) -> agents::delete::server::response::Frame;

    /// Tag 2. Send a message; resolves when it is delivered, or taken back
    /// in time. Firing `cancel` is the scope's `Cancel` channel request.
    async fn agents_message(
        &self,
        request: agents::message::client::request::Frame,
        cancel: CancellationToken,
    ) -> agents::message::server::response::Frame;

    /// Tag 3. Read a log, filtered, perhaps watched. Firing `cancel` ends a watch.
    fn agents_logs(
        &self,
        request: agents::logs::client::request::Frame,
        cancel: CancellationToken,
    ) -> Frames<agents::logs::server::response::Frame>;

    /// Tag 4. Every agent of the caller's, one each, then the scope finishes.
    fn agents_list(
        &self,
        request: agents::list::client::request::Frame,
    ) -> Frames<agents::list::server::response::Frame>;

    /// Tag 5. Change what an agent mounts: the three lists stated anew,
    /// each replacing the agent's list of that kind whole. Refused while
    /// the agent is active; its image, limits, machine and settings are
    /// its for life.
    async fn agents_edit(
        &self,
        request: agents::edit::client::request::Frame,
    ) -> agents::edit::server::response::Frame;

    /// Tag 6. Make a tool under a name: a tool container the daemon runs for
    /// the agents it's attached to, while one of them is active.
    async fn tools_create(&self, request: tools::create::client::request::Frame) -> tools::create::server::response::Frame;

    /// Tag 7. Change what a tool you created mounts, while it isn't running.
    async fn tools_edit(&self, request: tools::edit::client::request::Frame) -> tools::edit::server::response::Frame;

    /// Tag 8. Hold somebody else's running tool container under a name:
    /// the daemon joins it with the provider protocol's `tools::connect`.
    async fn tools_connect(&self, request: tools::connect::client::request::Frame) -> tools::connect::server::response::Frame;

    /// Tag 9. Put a tool among the MCP servers an agent's calls reach.
    async fn tools_attach(&self, request: tools::attach::client::request::Frame) -> tools::attach::server::response::Frame;

    /// Tag 10. Take it back, while the agent isn't active.
    async fn tools_detach(&self, request: tools::detach::client::request::Frame) -> tools::detach::server::response::Frame;

    /// Tag 11. Remove a tool attached nowhere.
    async fn tools_delete(&self, request: tools::delete::client::request::Frame) -> tools::delete::server::response::Frame;

    /// Tag 12. Every tool of the caller's, with the agents each is attached to.
    fn tools_list(&self, request: tools::list::client::request::Frame) -> Frames<tools::list::server::response::Frame>;

    // --- ours until the wire has them ---------------------------------

    async fn providers_list(&self) -> Vec<ProviderEntry>;
    async fn providers_add(&self, provider: NewProvider) -> Result<ProviderEntry, String>;
    async fn providers_remove(&self, identity: Identity) -> Result<(), String>;
}
