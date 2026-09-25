//! The seam between this app and the Diverge daemon.
//!
//! One trait, one method per verb `diverge-daemon-sdk` defines, taking and
//! returning that crate's own types. [`stub::StubDaemon`] stands in until
//! Ronald ships a daemon; a `WireDaemon` over the provider SDK's `client`
//! half is the whole of meeting up with him. Nothing above this module
//! knows which one it holds.
//!
//! Three methods are **ours until the wire has them** — `providers_*`. The
//! app adds machines inside itself (Maya, 2026-09-25); today a peer is a
//! `config.yaml` entry and the daemon has no verb for it. See
//! `QUESTIONS_FOR_RONALD.md`.

use std::pin::Pin;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use futures::Stream;
use tokio_util::sync::CancellationToken;

use diverge_daemon_sdk::endpoints::agents::logs::server::response::Identity;
use diverge_daemon_sdk::endpoints::{agents, filesystem};
use diverge_provider_sdk::shared::error::Error as WireError;

pub mod jq;
pub mod stub;

/// What a scope sends, in order, until it finishes.
pub type Frames<T> = Pin<Box<dyn Stream<Item = T> + Send + 'static>>;

/// `filesystem::read`'s answer, owned. The SDK's frame borrows the bytes
/// of the wire frame it was read from; a stream cannot hand that out.
#[derive(Debug, Clone)]
pub enum ReadFrame {
    Body(Vec<u8>),
    Error(WireError),
}

impl ReadFrame {
    /// Exhaustive on purpose: a new answer from the daemon breaks this.
    #[allow(dead_code)] // WireDaemon's; the stub builds ReadFrames itself.
    pub fn from_wire(frame: filesystem::read::server::response::Frame<'_>) -> Self {
        use filesystem::read::server::response::Frame;
        match frame {
            Frame::Body(body) => ReadFrame::Body(body.0.to_vec()),
            Frame::Error(error) => ReadFrame::Error(error),
        }
    }
}

/// A machine the daemon can run agents on. Ours until the wire has it.
#[derive(Debug, Clone)]
pub struct ProviderEntry {
    /// As the daemon names it — never anything the app decided.
    pub identity: Identity,
    /// Volume names on that provider. Ours: the daemon has no listing verb.
    pub volumes: Vec<String>,
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

    /// Tag 5. One file off the daemon's host, in pieces.
    fn filesystem_read(&self, request: filesystem::read::client::request::Frame) -> Frames<ReadFrame>;

    /// Tag 6. One file onto the daemon's host, put in place whole.
    async fn filesystem_write(
        &self,
        request: filesystem::write::client::request::Frame,
        body: Vec<u8>,
    ) -> filesystem::write::server::response::Frame;

    /// Tag 7. A directory's tree, then every change to it, until `cancel`.
    fn filesystem_filetree(
        &self,
        request: filesystem::filetree::client::request::Frame,
        cancel: CancellationToken,
    ) -> Frames<filesystem::filetree::server::response::Frame>;

    // --- ours until the wire has them ---------------------------------

    async fn providers_list(&self) -> Vec<ProviderEntry>;
    async fn providers_add(&self, provider: NewProvider) -> Result<ProviderEntry, String>;
    async fn providers_remove(&self, identity: Identity) -> Result<(), String>;
}
