//! The stand-in daemon: Ronald's verbs, answered locally, with his types.
//!
//! It keeps agents and their logs in memory, runs scripted "runs" (see
//! [`script`]), and serves `filesystem::*` from a real folder (see
//! [`host`]). It follows the SDK's documented semantics — spans inclusive,
//! the type and spans before the program, a watch that ends only when
//! nothing more can match, a delete that leaves an active agent alone, a
//! message that can be taken back only until it is delivered.

pub mod host;
pub mod script;

use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use async_trait::async_trait;
use chrono::{DateTime, TimeDelta, Utc};
use futures::stream;
use indexmap::IndexMap;
use rmcp::model::ContentBlock;
use serde_json::{Value, json};
use tokio::sync::{broadcast, mpsc, oneshot};
use tokio_util::sync::CancellationToken;

use diverge_daemon_sdk::endpoints::agents::create::client::request::{
    Frame as CreateRequest, Image, Provider as Pin, VolumeMount,
};
use diverge_daemon_sdk::endpoints::agents::logs::client::request::{Frame as LogsRequest, ItemType};
use diverge_daemon_sdk::endpoints::agents::logs::server::response::{
    Active, ActiveType, Error as ErrorItem, ErrorType, Identity, Inactive, InactiveType, Item, ItemWrapper, Provider,
};
use diverge_daemon_sdk::endpoints::{agents, filesystem};
use diverge_provider_sdk::endpoints::containers::agents::run::server::response::AgenticLoopChunk;
use diverge_provider_sdk::shared::containers::write_path;
use diverge_provider_sdk::shared::error::Error as WireError;
use diverge_provider_sdk::shared::filetree::response::Frame as TreeFrame;

use super::{Daemon, Frames, NewProvider, ProviderEntry, ReadFrame};
use crate::catalog::{Kind, UNBUILT_DIGEST};
use host::Host;
use script::Step;

/// How long a message waits before the agent takes it, when nothing is
/// ahead of it: long enough to take it back.
const DELIVERY: Duration = Duration::from_millis(2500);

fn wire_error(message: impl Into<String>) -> WireError {
    WireError(json!({ "message": message.into() }))
}

struct Queued {
    id: u64,
    content: Vec<ContentBlock>,
    delivered: oneshot::Sender<()>,
}

struct AgentState {
    create: CreateRequest,
    created: DateTime<Utc>,
    log: Vec<ItemWrapper>,
    active: Option<Provider>,
    queue: VecDeque<Queued>,
    running: bool,
    live: broadcast::Sender<ItemWrapper>,
    gone: CancellationToken,
}

impl AgentState {
    fn new(create: CreateRequest, created: DateTime<Utc>) -> Self {
        let (live, _) = broadcast::channel(1024);
        AgentState { create, created, log: Vec::new(), active: None, queue: VecDeque::new(), running: false, live, gone: CancellationToken::new() }
    }

    /// Keep an item; indexes count up from 1, so `0` is an empty log.
    fn keep(&mut self, created: DateTime<Utc>, item: Item) {
        let created = self.log.last().map_or(created, |last| created.max(last.created));
        let wrapper = ItemWrapper { logs_index: self.log.len() as u64 + 1, created, item };
        self.log.push(wrapper.clone());
        let _ = self.live.send(wrapper);
    }

    fn kind(&self) -> Option<Kind> {
        Kind::from_image_name(&self.create.image.name)
    }
}

struct ProviderState {
    identity: Identity,
    #[allow(dead_code)] // held as the daemon would; never shown
    key: String,
    volumes: Vec<String>,
    added: DateTime<Utc>,
}

struct Inner {
    agents: IndexMap<String, AgentState>,
    providers: Vec<ProviderState>,
    next_id: u64,
}

#[derive(Clone)]
pub struct StubDaemon {
    inner: Arc<Mutex<Inner>>,
    host: Arc<Host>,
    /// Its own work runs here, so a verb is safe to call from anywhere —
    /// a sync command on the main thread included.
    rt: tokio::runtime::Handle,
}

impl StubDaemon {
    /// Must be called inside a tokio runtime; that runtime is the one it keeps.
    pub fn new(host_root: std::path::PathBuf) -> Self {
        let daemon = StubDaemon {
            inner: Arc::new(Mutex::new(Inner { agents: IndexMap::new(), providers: Vec::new(), next_id: 1 })),
            host: Arc::new(Host::new(host_root)),
            rt: tokio::runtime::Handle::current(),
        };
        daemon.seed();
        daemon
    }

    pub fn host_root(&self) -> std::path::PathBuf {
        self.host.root().to_path_buf()
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, Inner> {
        self.inner.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    fn reader(&self) -> impl Fn(&str) -> String + use<> {
        let host = self.host.clone();
        move |path: &str| host.read_text(path)
    }

    /// The agent loop for one agent: take the next message, run, repeat;
    /// inactive once nothing waits. `first` is work already under way.
    fn spawn_runner(&self, name: String, first: Vec<Step>) {
        let daemon = self.clone();
        self.rt.spawn(async move { daemon.runner(name, first).await });
    }

    async fn play(&self, name: &str, steps: Vec<Step>, gone: &CancellationToken) -> bool {
        for step in steps {
            tokio::select! {
                _ = tokio::time::sleep(Duration::from_millis(step.delay_ms)) => {}
                _ = gone.cancelled() => return false,
            }
            let mut inner = self.lock();
            let Some(agent) = inner.agents.get_mut(name) else { return false };
            agent.keep(Utc::now(), step.item);
        }
        true
    }

    async fn runner(&self, name: String, first: Vec<Step>) {
        let Some(gone) = self.lock().agents.get(&name).map(|a| a.gone.clone()) else { return };
        if !first.is_empty() && !self.play(&name, first, &gone).await {
            return;
        }
        loop {
            tokio::select! {
                _ = tokio::time::sleep(DELIVERY) => {}
                _ = gone.cancelled() => return,
            }
            // Take the next message, or go inactive.
            let (message, kind, arguments, provider) = {
                let mut inner = self.lock();
                let first_provider = inner.providers.first().map(|p| p.identity.clone());
                let Some(agent) = inner.agents.get_mut(&name) else { return };
                let Some(message) = agent.queue.pop_front() else {
                    if let Some(provider) = agent.active.take() {
                        agent.keep(Utc::now(), Item::Inactive(Inactive { r#type: InactiveType::Inactive, provider }));
                    }
                    agent.running = false;
                    return;
                };
                let provider = agent.create.provider.as_ref().map(|p| p.identity.clone()).or(first_provider);
                (message, agent.kind(), agent.create.arguments.clone(), provider)
            };
            let _ = message.delivered.send(());
            let text = keep_user_parts(self, &name, message.id, &message.content);

            let Some(identity) = provider else {
                let mut inner = self.lock();
                if let Some(agent) = inner.agents.get_mut(&name) {
                    agent.keep(Utc::now(), error_item("no provider to run on: add a machine first"));
                }
                continue;
            };
            {
                let mut inner = self.lock();
                let Some(agent) = inner.agents.get_mut(&name) else { return };
                if agent.active.is_none() {
                    let provider = Provider { identity };
                    agent.keep(Utc::now(), Item::Active(Active { r#type: ActiveType::Active, provider: provider.clone() }));
                    agent.active = Some(provider);
                }
            }
            let steps = match kind {
                None => vec![Step { delay_ms: 400, item: error_item("no provider supplies this image") }],
                Some(kind) => match kind.check(&arguments) {
                    Err(reason) => vec![Step { delay_ms: 600, item: error_item(format!("these settings are not a {} agent: {reason}", kind.key())) }],
                    Ok(()) => script::run(kind, &text, &self.reader()),
                },
            };
            if !self.play(&name, steps, &gone).await {
                return;
            }
        }
    }

    fn seed(&self) {
        let now = Utc::now();
        let here = Identity::Outgoing { address: "127.0.0.1:4640".into() };
        let studio = Identity::IncomingUnbrokered { identity: "studio-pc".into() };
        {
            let mut inner = self.lock();
            inner.providers.push(ProviderState { identity: here.clone(), key: "stand-in".into(), volumes: vec!["workspace".into(), "datasets".into()], added: now - TimeDelta::days(9) });
            inner.providers.push(ProviderState { identity: studio.clone(), key: "stand-in".into(), volumes: vec!["media".into()], added: now - TimeDelta::days(3) });
        }
        let image = |kind: Kind| Image { name: kind.image_name(), digest: UNBUILT_DIGEST.into() };
        let create = |kind: Kind, name: &str, arguments: Value, provider: Option<Pin>| CreateRequest {
            image: image(kind),
            memory: 4 << 30,
            disk: 8 << 30,
            provider,
            fuse_file_mounts: Vec::new(),
            fuse_directory_mounts: Vec::new(),
            arguments,
            name: name.into(),
        };
        let read = self.reader();

        // A finished run from yesterday.
        let mut notes = AgentState::new(create(Kind::Hermes, "research-notes", json!({ "provider": { "provider": "openrouter", "api_key": "stand-in" }, "model": "nousresearch/hermes-4-70b" }), None), now - TimeDelta::days(2));
        let mut t = now - TimeDelta::hours(20);
        notes.keep(t, user_text(1, "Pull the open questions out of my notes"));
        notes.keep(t, Item::Active(Active { r#type: ActiveType::Active, provider: Provider { identity: here.clone() } }));
        for step in script::run(Kind::Hermes, "Pull the open questions out of my notes", &read) {
            t += TimeDelta::milliseconds(step.delay_ms as i64);
            notes.keep(t, step.item);
        }
        notes.keep(t + TimeDelta::seconds(1), Item::Inactive(Inactive { r#type: InactiveType::Inactive, provider: Provider { identity: here.clone() } }));

        // A run that failed.
        let mut broken = AgentState::new(create(Kind::Openrouter, "broken-loop", json!({ "model": "openrouter/auto" }), None), now - TimeDelta::hours(6));
        let t = now - TimeDelta::hours(5);
        broken.keep(t, user_text(2, "Summarise the reading list"));
        broken.keep(t, Item::Active(Active { r#type: ActiveType::Active, provider: Provider { identity: studio.clone() } }));
        broken.keep(t + TimeDelta::seconds(2), script::chunk(json!({ "type": "assistant_reasoning", "text": "Reading the list first." })));
        broken.keep(t + TimeDelta::seconds(4), error_item("the upstream refused the request: no API key for it in the vault"));
        broken.keep(t + TimeDelta::seconds(4), Item::Inactive(Inactive { r#type: InactiveType::Inactive, provider: Provider { identity: studio.clone() } }));

        // One mid-way through a long job, pinned to this machine's workspace.
        let pin = Pin { identity: here.clone(), volume_mounts: vec![VolumeMount { volume_name: "workspace".into(), volume_relative_path: vec![], container_path: vec!["workspace".into()] }] };
        let mut site = AgentState::new(create(Kind::Cc, "site-fixes", json!({ "model": "claude-sonnet-5", "tools": { "read": true, "glob": true, "grep": true, "bash": true, "task": true } }), Some(pin)), now - TimeDelta::minutes(40));
        let t = now - TimeDelta::seconds(20);
        site.keep(t, user_text(3, "Go through the whole site and tidy it, section by section."));
        site.keep(t, Item::Active(Active { r#type: ActiveType::Active, provider: Provider { identity: here.clone() } }));
        site.active = Some(Provider { identity: here });
        site.running = true;

        {
            let mut inner = self.lock();
            inner.next_id = 4;
            inner.agents.insert("site-fixes".into(), site);
            inner.agents.insert("research-notes".into(), notes);
            inner.agents.insert("broken-loop".into(), broken);
        }
        self.spawn_runner("site-fixes".into(), script::long_job(&read));
    }
}

fn user_text(id: u64, text: &str) -> Item {
    script::chunk(json!({ "type": "user_text_content", "key": format!("m{id}"), "text": text }))
}

fn error_item(message: impl Into<String>) -> Item {
    Item::Error(ErrorItem { r#type: ErrorType::Error, error: wire_error(message) })
}

/// Keep a delivered message's parts as user chunks; the text, for the run.
fn keep_user_parts(daemon: &StubDaemon, name: &str, id: u64, content: &[ContentBlock]) -> String {
    let mut text = String::new();
    let mut inner = daemon.lock();
    let Some(agent) = inner.agents.get_mut(name) else { return text };
    for block in content {
        let value = serde_json::to_value(block).unwrap_or(Value::Null);
        let kind = value.get("type").and_then(Value::as_str).unwrap_or_default().to_owned();
        let mut part = value.clone();
        if let Some(object) = part.as_object_mut() {
            object.remove("type");
            object.insert("key".into(), json!(format!("m{id}")));
        }
        let part_type = match kind.as_str() {
            "text" => {
                text.push_str(value.get("text").and_then(Value::as_str).unwrap_or_default());
                "user_text_content"
            }
            "image" => "user_image_content",
            "audio" => "user_audio_content",
            "resource" => "user_resource",
            "resource_link" => "user_resource_link",
            _ => continue,
        };
        part["type"] = json!(part_type);
        if let Ok(chunk) = serde_json::from_value::<AgenticLoopChunk>(part) {
            agent.keep(Utc::now(), Item::Chunk(chunk));
        }
    }
    text
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A sync Tauri command runs on the main thread, outside any runtime.
    /// Draft one crashed here once; never again.
    #[test]
    fn verbs_work_from_a_thread_with_no_runtime() {
        use futures::StreamExt;
        let rt = tokio::runtime::Runtime::new().unwrap();
        let root = std::env::temp_dir().join(format!("diverge-desktop-test-rt-{}", std::process::id()));
        let daemon = rt.block_on(async { StubDaemon::new(root) });
        let request = LogsRequest { name: "research-notes".into(), logs_index_from: None, logs_index_to: None, created_from: None, created_to: None, r#type: None, jq: None, count: Some(3), watch: None };
        let frames = std::thread::spawn(move || daemon.agents_logs(request, CancellationToken::new())).join().unwrap();
        let got = rt.block_on(frames.collect::<Vec<_>>());
        assert_eq!(got.len(), 3);
    }

    #[tokio::test]
    async fn machines_are_added_both_ways_and_removed_unless_busy() {
        let root = std::env::temp_dir().join(format!("diverge-desktop-test-m-{}", std::process::id()));
        let daemon = StubDaemon::new(root);
        let dial = daemon.providers_add(NewProvider::Dial { address: "studio.local:4640".into(), key: "k".into() }).await.unwrap();
        assert_eq!(dial.identity, Identity::Outgoing { address: "studio.local:4640".into() });
        daemon.providers_add(NewProvider::Accept { identity: "friend".into(), key: "k".into() }).await.unwrap();
        assert!(daemon.providers_add(NewProvider::Accept { identity: "friend".into(), key: "k".into() }).await.is_err(), "no duplicates");
        assert!(daemon.providers_add(NewProvider::Dial { address: "x:1".into(), key: " ".into() }).await.is_err(), "a key is needed");
        assert_eq!(daemon.providers_list().await.len(), 4);
        // site-fixes is running on 127.0.0.1:4640, so that one stays.
        assert!(daemon.providers_remove(Identity::Outgoing { address: "127.0.0.1:4640".into() }).await.is_err());
        daemon.providers_remove(Identity::IncomingUnbrokered { identity: "friend".into() }).await.unwrap();
        assert_eq!(daemon.providers_list().await.len(), 3);
    }

    #[tokio::test]
    async fn seeded_agents_have_settings_their_image_accepts() {
        let root = std::env::temp_dir().join(format!("diverge-desktop-test-{}", std::process::id()));
        let daemon = StubDaemon::new(root);
        let inner = daemon.lock();
        for (name, agent) in &inner.agents {
            let kind = agent.kind().expect("a seeded agent names one of the six images");
            assert!(kind.check(&agent.create.arguments).is_ok(), "{name}: {:?}", kind.check(&agent.create.arguments));
        }
    }
}

/// The `type` an item carries, as a logs request names one.
fn item_type(item: &Item) -> ItemType {
    match item {
        Item::Chunk(chunk) => match chunk {
            AgenticLoopChunk::AssistantReasoning(_) => ItemType::AssistantReasoning,
            AgenticLoopChunk::AssistantTextContent(_) => ItemType::AssistantTextContent,
            AgenticLoopChunk::AssistantImageContent(_) => ItemType::AssistantImageContent,
            AgenticLoopChunk::AssistantAudioContent(_) => ItemType::AssistantAudioContent,
            AgenticLoopChunk::AssistantToolCall(_) => ItemType::AssistantToolCall,
            AgenticLoopChunk::AssistantRefusal(_) => ItemType::AssistantRefusal,
            AgenticLoopChunk::ToolResponse(_) => ItemType::ToolResponse,
            AgenticLoopChunk::UserTextContent(_) => ItemType::UserTextContent,
            AgenticLoopChunk::UserImageContent(_) => ItemType::UserImageContent,
            AgenticLoopChunk::UserAudioContent(_) => ItemType::UserAudioContent,
            AgenticLoopChunk::UserResource(_) => ItemType::UserResource,
            AgenticLoopChunk::UserResourceLink(_) => ItemType::UserResourceLink,
            AgenticLoopChunk::Usage(_) => ItemType::Usage,
            AgenticLoopChunk::Notification(_) => ItemType::Notification,
        },
        Item::Error(_) => ItemType::Error,
        Item::Active(_) => ItemType::Active,
        Item::Inactive(_) => ItemType::Inactive,
    }
}

/// The spans and the type: what the program is allowed to see.
fn in_filter(request: &LogsRequest, item: &ItemWrapper) -> bool {
    request.logs_index_from.is_none_or(|from| item.logs_index >= from)
        && request.logs_index_to.is_none_or(|to| item.logs_index <= to)
        && request.created_from.is_none_or(|from| item.created >= from)
        && request.created_to.is_none_or(|to| item.created <= to)
        && request.r#type.is_none_or(|kind| item_type(&item.item) == kind)
}

/// What comes back for one item that passed the filter.
fn yields(request: &LogsRequest, item: &ItemWrapper) -> Result<Vec<Value>, String> {
    let value = serde_json::to_value(item).map_err(|e| e.to_string())?;
    match &request.jq {
        None => Ok(vec![value]),
        Some(program) => super::jq::run(value, program),
    }
}

#[async_trait]
impl Daemon for StubDaemon {
    async fn agents_create(&self, request: CreateRequest) -> agents::create::server::response::Frame {
        use agents::create::server::response::Frame;
        if request.name.trim().is_empty() {
            return Frame::Error(wire_error("an agent needs a name"));
        }
        for mount in request.fuse_file_mounts.iter().chain(&request.fuse_directory_mounts) {
            if !self.host.exists(&mount.daemon_path) {
                return Frame::Error(wire_error(format!("nothing on this machine at {}", mount.daemon_path)));
            }
        }
        let mut inner = self.lock();
        if inner.agents.contains_key(&request.name) {
            return Frame::InUse;
        }
        if let Some(pin) = &request.provider {
            let Some(provider) = inner.providers.iter().find(|p| p.identity == pin.identity) else {
                return Frame::Error(wire_error("the daemon knows no provider by that identity"));
            };
            for mount in &pin.volume_mounts {
                if !provider.volumes.contains(&mount.volume_name) {
                    return Frame::Error(wire_error(format!("that provider has no volume named \"{}\"", mount.volume_name)));
                }
            }
        }
        let name = request.name.clone();
        inner.agents.insert(name, AgentState::new(request, Utc::now()));
        Frame::Created
    }

    async fn agents_delete(&self, request: agents::delete::client::request::Frame) -> agents::delete::server::response::Frame {
        use agents::delete::server::response::Frame;
        let mut inner = self.lock();
        match inner.agents.get(&request.name) {
            None => Frame::NotFound,
            Some(agent) if agent.active.is_some() || agent.running => Frame::Active,
            Some(_) => {
                if let Some(agent) = inner.agents.shift_remove(&request.name) {
                    agent.gone.cancel();
                }
                Frame::Deleted
            }
        }
    }

    async fn agents_message(
        &self,
        request: agents::message::client::request::Frame,
        cancel: CancellationToken,
    ) -> agents::message::server::response::Frame {
        use agents::message::server::response::Frame;
        let (delivered, mut on_delivery) = oneshot::channel();
        let (id, start) = {
            let mut inner = self.lock();
            let id = inner.next_id;
            inner.next_id += 1;
            let Some(agent) = inner.agents.get_mut(&request.name) else {
                return Frame::Error(wire_error(format!("no agent named \"{}\"", request.name)));
            };
            agent.queue.push_back(Queued { id, content: request.content, delivered });
            let start = !agent.running;
            agent.running = true;
            (id, start)
        };
        if start {
            self.spawn_runner(request.name.clone(), Vec::new());
        }
        tokio::select! {
            result = &mut on_delivery => match result {
                Ok(()) => Frame::Delivered,
                Err(_) => Frame::Error(wire_error("the agent was deleted")),
            },
            _ = cancel.cancelled() => {
                {
                    let mut inner = self.lock();
                    if let Some(agent) = inner.agents.get_mut(&request.name) {
                        if let Some(at) = agent.queue.iter().position(|q| q.id == id) {
                            agent.queue.remove(at);
                            return Frame::Cancelled;
                        }
                    }
                }
                match on_delivery.await {
                    Ok(()) => Frame::Delivered,
                    Err(_) => Frame::Error(wire_error("the agent was deleted")),
                }
            }
        }
    }

    fn agents_logs(&self, request: LogsRequest, cancel: CancellationToken) -> Frames<agents::logs::server::response::Frame> {
        use agents::logs::server::response::Frame;
        let (tx, rx) = mpsc::channel::<Frame>(512);
        let daemon = self.clone();
        self.rt.spawn(async move {
            let found = {
                let inner = daemon.lock();
                inner.agents.get(&request.name).map(|agent| (agent.log.clone(), agent.live.subscribe(), agent.gone.clone()))
            };
            let Some((history, mut live, gone)) = found else {
                let _ = tx.send(Frame::Error(wire_error(format!("no agent named \"{}\"", request.name)))).await;
                return;
            };
            let limit = request.count;
            let mut sent: u64 = 0;
            if limit == Some(0) {
                return;
            }
            // Returns false once the scope should finish.
            let send = async |item: &ItemWrapper, sent: &mut u64| -> bool {
                if !in_filter(&request, item) {
                    return true;
                }
                match yields(&request, item) {
                    Err(reason) => {
                        let _ = tx.send(Frame::Error(wire_error(reason))).await;
                        false
                    }
                    Ok(values) => {
                        for value in values {
                            if tx.send(Frame::Value(value)).await.is_err() {
                                return false;
                            }
                            *sent += 1;
                            if limit.is_some_and(|limit| *sent >= limit) {
                                return false;
                            }
                        }
                        true
                    }
                }
            };
            let mut last = 0;
            for item in &history {
                last = item.logs_index;
                if !send(item, &mut sent).await {
                    return;
                }
            }
            if request.watch != Some(true) {
                return;
            }
            let past_span = |item: &ItemWrapper| {
                request.logs_index_to.is_some_and(|to| item.logs_index >= to)
                    || request.created_to.is_some_and(|to| item.created > to)
            };
            if history.last().is_some_and(past_span) {
                return;
            }
            let deadline = request.created_to.map(|to| (to - Utc::now()).to_std().unwrap_or_default());
            let clock = async {
                match deadline {
                    Some(wait) => tokio::time::sleep(wait).await,
                    None => futures::future::pending::<()>().await,
                }
            };
            tokio::pin!(clock);
            loop {
                tokio::select! {
                    _ = cancel.cancelled() => return,
                    _ = gone.cancelled() => return,
                    _ = &mut clock => return,
                    next = live.recv() => match next {
                        Ok(item) => {
                            if item.logs_index <= last {
                                continue;
                            }
                            last = item.logs_index;
                            if !send(&item, &mut sent).await || past_span(&item) {
                                return;
                            }
                        }
                        Err(broadcast::error::RecvError::Lagged(_)) => continue,
                        Err(broadcast::error::RecvError::Closed) => return,
                    },
                }
            }
        });
        Box::pin(stream::unfold(rx, |mut rx| async move { rx.recv().await.map(|frame| (frame, rx)) }))
    }

    fn agents_list(&self, _request: agents::list::client::request::Frame) -> Frames<agents::list::server::response::Frame> {
        use agents::list::server::response::{Agent, Frame};
        let inner = self.lock();
        let frames: Vec<Frame> = inner
            .agents
            .iter()
            .map(|(name, agent)| {
                let last_change = agent.log.iter().rev().find_map(|item| match &item.item {
                    Item::Active(Active { provider, .. }) | Item::Inactive(Inactive { provider, .. }) => Some((item.created, provider.clone())),
                    Item::Chunk(_) | Item::Error(_) => None,
                });
                Frame::Agent(Agent {
                    name: name.clone(),
                    image: agent.create.image.clone(),
                    created: agent.created,
                    active: agent.active.is_some(),
                    last_active: last_change.as_ref().map(|(at, _)| *at),
                    provider: last_change.map(|(_, provider)| provider),
                    logs_index: agent.log.last().map_or(0, |item| item.logs_index),
                })
            })
            .collect();
        Box::pin(stream::iter(frames))
    }

    fn filesystem_read(&self, request: filesystem::read::client::request::Frame) -> Frames<ReadFrame> {
        let frames = match self.host.read(&request.path) {
            Err(reason) => vec![ReadFrame::Error(wire_error(reason))],
            Ok(bytes) if bytes.is_empty() => Vec::new(),
            Ok(bytes) => bytes.chunks(64 * 1024).map(|piece| ReadFrame::Body(piece.to_vec())).collect(),
        };
        Box::pin(stream::iter(frames))
    }

    async fn filesystem_write(
        &self,
        request: filesystem::write::client::request::Frame,
        body: Vec<u8>,
    ) -> filesystem::write::server::response::Frame {
        use filesystem::write::server::response::Frame;
        match self.host.write(&request.path, &body) {
            Ok(()) => Frame::Written(write_path::response::Frame),
            Err(reason) => Frame::Error(wire_error(reason)),
        }
    }

    fn filesystem_filetree(
        &self,
        request: filesystem::filetree::client::request::Frame,
        cancel: CancellationToken,
    ) -> Frames<filesystem::filetree::server::response::Frame> {
        use filesystem::filetree::server::response::Frame;
        use notify::{RecursiveMode, Watcher};
        let (tx, rx) = mpsc::channel::<Frame>(256);
        let host = self.host.clone();
        self.rt.spawn(async move {
            let mut tree = match host.tree(&request.path) {
                Ok(tree) => tree,
                Err(reason) => {
                    let _ = tx.send(Frame::Error(wire_error(reason))).await;
                    return;
                }
            };
            if tx.send(Frame::Filetree(TreeFrame::Snapshot { children: tree.clone() })).await.is_err() {
                return;
            }
            let (events_tx, mut events) = mpsc::unbounded_channel();
            let Ok(dir) = host.resolve(&request.path) else { return };
            let mut watcher = match notify::recommended_watcher(move |event| {
                let _ = events_tx.send(event);
            }) {
                Ok(watcher) => watcher,
                Err(_) => return,
            };
            if watcher.watch(&dir, RecursiveMode::Recursive).is_err() {
                return;
            }
            loop {
                tokio::select! {
                    _ = cancel.cancelled() => return,
                    event = events.recv() => {
                        if event.is_none() {
                            return;
                        }
                        // Let a burst settle, then report what changed.
                        tokio::time::sleep(Duration::from_millis(150)).await;
                        while events.try_recv().is_ok() {}
                        let Ok(next) = host.tree(&request.path) else { return };
                        let mut frames = Vec::new();
                        host::diff(&[], &tree, &next, &mut frames);
                        tree = next;
                        for frame in frames {
                            if tx.send(Frame::Filetree(frame)).await.is_err() {
                                return;
                            }
                        }
                    }
                }
            }
        });
        Box::pin(stream::unfold(rx, |mut rx| async move { rx.recv().await.map(|frame| (frame, rx)) }))
    }

    async fn providers_list(&self) -> Vec<ProviderEntry> {
        self.lock()
            .providers
            .iter()
            .map(|p| ProviderEntry { identity: p.identity.clone(), volumes: p.volumes.clone(), added: p.added })
            .collect()
    }

    async fn providers_add(&self, provider: NewProvider) -> Result<ProviderEntry, String> {
        let (identity, key) = match provider {
            NewProvider::Dial { address, key } => {
                let address = address.trim().to_owned();
                if address.is_empty() {
                    return Err("an address is needed".into());
                }
                (Identity::Outgoing { address }, key)
            }
            NewProvider::Accept { identity, key } => {
                let identity = identity.trim().to_owned();
                if identity.is_empty() {
                    return Err("a name is needed".into());
                }
                (Identity::IncomingUnbrokered { identity }, key)
            }
        };
        if key.trim().is_empty() {
            return Err("a key is needed".into());
        }
        let mut inner = self.lock();
        if inner.providers.iter().any(|p| p.identity == identity) {
            return Err("the daemon already knows that machine".into());
        }
        let entry = ProviderState { identity, key, volumes: Vec::new(), added: Utc::now() };
        let out = ProviderEntry { identity: entry.identity.clone(), volumes: Vec::new(), added: entry.added };
        inner.providers.push(entry);
        Ok(out)
    }

    async fn providers_remove(&self, identity: Identity) -> Result<(), String> {
        let mut inner = self.lock();
        let running_there = inner.agents.values().any(|a| a.active.as_ref().is_some_and(|p| p.identity == identity));
        if running_there {
            return Err("an agent is running there right now".into());
        }
        let before = inner.providers.len();
        inner.providers.retain(|p| p.identity != identity);
        if inner.providers.len() == before {
            return Err("the daemon doesn't know that machine".into());
        }
        Ok(())
    }
}
