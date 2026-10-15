//! The seam between this app and the Diverge daemon.
//!
//! One trait, one method per verb `diverge_sdk::daemon` defines that the
//! app uses, taking and returning that module's own types.
//! `stub::StubDaemon` (feature `stand-in`) stands in until a daemon ships;
//! a `WireDaemon` over the SDK's caller half is the whole of meeting up
//! with him. Nothing above this module knows which one it holds.
//!
//! An agent is made from a template (`agents::templates`), held by its
//! hash, and named by a [`reference::Agent`]: its name if it has one, or
//! its template and index, which always work. The app keys agents by
//! [`agent_key`] and turns a key back into a reference with
//! [`reference_of`].
//!
//! Since `e28031bbd` the daemon has no volumes: they are each machine's,
//! behind the other seam, [`crate::machines`].
//!
//! Three methods are **ours until the wire has them** — `providers_*`. The
//! app adds machines inside itself (a product ruling, 2026-09-25); today a peer is a
//! `config.yaml` entry and the daemon has no verb for it.

use std::pin::Pin;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use futures::{Stream, StreamExt};
use serde::Serialize;
use sha2::{Digest, Sha256};
use tokio_util::sync::CancellationToken;

use diverge_sdk::daemon::endpoints::agents::logs::server::response::Identity;
use diverge_sdk::daemon::endpoints::{agents, standalone_tools};
use diverge_sdk::daemon::reference;
use diverge_sdk::daemon::template::Template;

#[cfg(feature = "stand-in")]
pub mod stub;

/// What a scope sends, in order, until it finishes.
pub type Frames<T> = Pin<Box<dyn Stream<Item = T> + Send + 'static>>;

/// A template's id, as the daemon answers it: the SHA-256, in lowercase
/// hex, of the template's canonical bytes with the image's references
/// left out (`daemon::template`'s own rule).
pub fn template_id<T: Serialize + Clone>(template: &Template<T>) -> String {
    let bytes = diverge_sdk::shared::canonical::bytes(&template.hashed()).unwrap_or_default();
    hex::encode(Sha256::digest(bytes))
}

/// How the app keys an agent: its name when it has one, otherwise its
/// template and its index, which name it once and for all.
pub fn agent_key(name: Option<&str>, template: &str, index: u64) -> String {
    match name {
        Some(name) => name.to_owned(),
        None => format!("{template}#{index}"),
    }
}

/// A key back into the daemon's reference: a template's id and an index
/// for an agent with no name, its name otherwise.
pub fn reference_of(key: &str) -> reference::Agent {
    match key.rsplit_once('#') {
        Some((template, index)) if template.len() == 64 && template.bytes().all(|b| b.is_ascii_hexdigit()) => match index.parse() {
            Ok(index) => reference::Agent::TemplateIndex { template: template.to_owned(), index },
            Err(_) => reference::Agent::Name { name: key.to_owned() },
        },
        _ => reference::Agent::Name { name: key.to_owned() },
    }
}

/// The agents list read once: everything listed when the scope opened, up
/// to and including `Listed` (or the refusal or failure that ends it), then
/// the scope is cancelled. The list itself stays open on the wire; the app
/// follows it live past `Listed` in one place, [`crate::fleet`].
pub async fn agents_listed(daemon: &dyn Daemon) -> Vec<agents::list::server::response::Frame> {
    use agents::list::server::response::Frame;
    let cancel = CancellationToken::new();
    let request = agents::list::client::request::Frame { filter: Default::default(), count: None, relay: None };
    let mut frames = daemon.agents_list(request, cancel.clone());
    let mut out = Vec::new();
    while let Some(frame) = frames.next().await {
        let done = match &frame {
            Frame::Added(_) | Frame::Changed(_) | Frame::Removed(_) => false,
            Frame::Listed | Frame::Forbidden | Frame::Error(_) => true,
        };
        out.push(frame);
        if done {
            break;
        }
    }
    cancel.cancel();
    out
}

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
    /// Tag 9. Hold a template agents are made from; answers its id. The
    /// same template twice is one template, answered `Exists`.
    async fn agents_templates_create(
        &self,
        request: agents::templates::create::client::request::Frame,
    ) -> agents::templates::create::server::response::Frame;

    /// Tag 10. One template, by its id.
    async fn agents_templates_get(
        &self,
        request: agents::templates::get::client::request::Frame,
    ) -> agents::templates::get::server::response::Frame;

    /// Tag 0. Spawn an agent from a template, perhaps under a name.
    async fn agents_create(
        &self,
        request: agents::create::client::request::Frame,
    ) -> agents::create::server::response::Frame;

    /// Tag 2. Delete an agent. An active agent is left as it is.
    async fn agents_delete(
        &self,
        request: agents::delete::client::request::Frame,
    ) -> agents::delete::server::response::Frame;

    /// Tag 3. Send a message; resolves when it is delivered, or taken back
    /// in time. Firing `cancel` is the scope's `Cancel` channel request.
    async fn agents_message(
        &self,
        request: agents::message::client::request::Frame,
        cancel: CancellationToken,
    ) -> agents::message::server::response::Frame;

    /// Tag 4. Read a log, filtered, perhaps watched. Firing `cancel` ends a watch.
    fn agents_logs(
        &self,
        request: agents::logs::client::request::Frame,
        cancel: CancellationToken,
    ) -> Frames<agents::logs::server::response::Frame>;

    /// Tag 5. The caller's agents: one `Added` each, then `Listed`, then
    /// what changes, until `cancel` fires (the scope's `Cancel`).
    /// [`agents_listed`] reads it once.
    fn agents_list(
        &self,
        request: agents::list::client::request::Frame,
        cancel: CancellationToken,
    ) -> Frames<agents::list::server::response::Frame>;

    /// Tag 6. Change an agent: what it mounts, each list stated anew and
    /// replacing the agent's list of that kind whole (refused while the
    /// agent is active), or its name. Its template and machine are its
    /// for life.
    async fn agents_edit(
        &self,
        request: agents::edit::client::request::Frame,
    ) -> agents::edit::server::response::Frame;

    /// Tag 7. Put tags on an agent of yours; one it holds already is held
    /// still. Its list item reports them, sorted.
    async fn agents_tag(&self, request: agents::tag::client::request::Frame) -> agents::tag::server::response::Frame;

    /// Tag 8. Take tags off an agent of yours; one it doesn't hold is no failure.
    async fn agents_untag(&self, request: agents::untag::client::request::Frame) -> agents::untag::server::response::Frame;

    // The tools verbs. The app reaches no room through them: a tool the
    // daemon attaches bypasses the app, which seals agents' calls and asks
    // the person first. Only the stand-in's own tests call them yet.

    /// Tag 25. Hold a template tools are made from; answers its id.
    #[allow(dead_code)]
    async fn standalone_tools_templates_create(
        &self,
        request: standalone_tools::templates::create::client::request::Frame,
    ) -> standalone_tools::templates::create::server::response::Frame;

    /// Tag 15. Make a tool from a template, perhaps under a name: a tool
    /// container the daemon runs for the agents it's attached to.
    #[allow(dead_code)]
    async fn standalone_tools_create(&self, request: standalone_tools::create::client::request::Frame) -> standalone_tools::create::server::response::Frame;

    /// Tag 17. Change a tool you created: its mounts, while it isn't running, or its name.
    #[allow(dead_code)]
    async fn standalone_tools_edit(&self, request: standalone_tools::edit::client::request::Frame) -> standalone_tools::edit::server::response::Frame;

    /// Tag 18. Keep a record of another daemon's tool, perhaps under a name.
    #[allow(dead_code)]
    async fn standalone_tools_register(&self, request: standalone_tools::register::client::request::Frame) -> standalone_tools::register::server::response::Frame;

    /// Tag 19. Put a tool among the MCP servers one agent's calls reach.
    #[allow(dead_code)]
    async fn standalone_tools_attach_agent_instance(
        &self,
        request: standalone_tools::attach_agent_instance::client::request::Frame,
    ) -> standalone_tools::attach_agent_instance::server::response::Frame;

    /// Tag 20. Take it back from that agent.
    #[allow(dead_code)]
    async fn standalone_tools_detach_agent_instance(
        &self,
        request: standalone_tools::detach_agent_instance::client::request::Frame,
    ) -> standalone_tools::detach_agent_instance::server::response::Frame;

    /// Tag 101. Put a tool among the MCP servers of every agent made from a template.
    #[allow(dead_code)]
    async fn standalone_tools_attach_agent_template(
        &self,
        request: standalone_tools::attach_agent_template::client::request::Frame,
    ) -> standalone_tools::attach_agent_template::server::response::Frame;

    /// Tag 102. Take it back from that template.
    #[allow(dead_code)]
    async fn standalone_tools_detach_agent_template(
        &self,
        request: standalone_tools::detach_agent_template::client::request::Frame,
    ) -> standalone_tools::detach_agent_template::server::response::Frame;

    /// Tag 21. Remove a tool attached nowhere.
    #[allow(dead_code)]
    async fn standalone_tools_delete(&self, request: standalone_tools::delete::client::request::Frame) -> standalone_tools::delete::server::response::Frame;

    /// Tag 22. The caller's tools, `Added` each then `Listed`, open until `cancel` fires.
    #[allow(dead_code)]
    fn standalone_tools_list(
        &self,
        request: standalone_tools::list::client::request::Frame,
        cancel: CancellationToken,
    ) -> Frames<standalone_tools::list::server::response::Frame>;

    // --- ours until the wire has them ---------------------------------

    async fn providers_list(&self) -> Vec<ProviderEntry>;
    async fn providers_add(&self, provider: NewProvider) -> Result<ProviderEntry, String>;
    async fn providers_remove(&self, identity: Identity) -> Result<(), String>;
}
