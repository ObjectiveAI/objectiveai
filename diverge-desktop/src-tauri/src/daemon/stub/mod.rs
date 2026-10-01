//! The stand-in daemon: the daemon's verbs, answered locally, with the SDK's types.
//!
//! It keeps agents and their logs in memory, runs scripted "runs" (see
//! [`script`]), and answers for every machine it knows as well: each
//! machine's volumes are real folders (see [`store`]), behind the other
//! seam, [`crate::machines`]. It follows the SDK's documented semantics —
//! spans inclusive, the type and spans before the program, a watch that
//! ends only when nothing more can match, a delete or an edit that leaves
//! an active agent alone, a message that can be taken back only until it
//! is delivered, a volume nothing touches while anything holds it, and a
//! volume that keeps its changes with one user at a time.

pub mod script;
pub mod store;

use std::collections::VecDeque;
use std::path::PathBuf;
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

use diverge_sdk::daemon::endpoints::agents::create::client::request::{
    Frame as CreateRequest, FuseMount, Image, Provider as Pin, VolumeMount,
};
use diverge_sdk::daemon::endpoints::agents::logs::client::request::{Frame as LogsRequest, ItemType};
use diverge_sdk::daemon::endpoints::agents::logs::server::response::{
    Active, ActiveType, Error as ErrorItem, ErrorType, Identity, Inactive, InactiveType, Item, ItemWrapper, Provider,
};
use diverge_sdk::daemon::endpoints::{agents, tools};
use diverge_sdk::provider::endpoints::containers::agents::run::server::response::AgenticLoopChunk;
use diverge_sdk::provider::endpoints::volumes;
use diverge_sdk::provider::endpoints::volumes::Mode;
use diverge_sdk::provider::endpoints::volumes::edit::client::request::Change;
use diverge_sdk::shared::containers::write_path;
use diverge_sdk::shared::error::Error as WireError;

use super::{Daemon, Frames, NewProvider, ProviderEntry};
use crate::machines::{Machines, ReadFrame};
use crate::catalog::{Kind, UNBUILT_DIGEST};
use script::{Piece, Step};
use store::VolumeStore;
use crate::door::Door;
use std::sync::OnceLock;

/// Present in any build the stand-in is part of: the check that it's absent looks for these words.
pub const BUILT_IN: &str = "diverge-desktop stand-in daemon is built in";

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
    /// The machine's own disk, as far as its volumes go.
    store: Arc<VolumeStore>,
    added: DateTime<Utc>,
}

/// A machine as a person reads it.
fn show(identity: &Identity) -> &str {
    match identity {
        Identity::Outgoing { address } => address,
        Identity::IncomingUnbrokered { identity } => identity,
    }
}

/// A machine's folder name under the stand-in's data folder.
fn slug(identity: &Identity) -> String {
    let raw = match identity {
        Identity::Outgoing { address } => format!("dial-{address}"),
        Identity::IncomingUnbrokered { identity } => format!("accept-{identity}"),
    };
    raw.chars().map(|c| if c.is_ascii_alphanumeric() || c == '-' || c == '.' { c } else { '_' }).collect()
}

/// Who holds a machine's volume, if anyone. An agent that mounts it over
/// FUSE holds it for its life: the daemon's serve runs from the create to
/// the delete. An agent pinned there with it mounted holds it while a run
/// is under way. Nothing examines, reads, writes, walks, resizes or
/// deletes a volume while anything holds it.
fn holder(inner: &Inner, on: &Identity, volume: &str, except: Option<&str>) -> Option<String> {
    inner
        .agents
        .iter()
        .filter(|(name, _)| Some(name.as_str()) != except)
        .find(|(_, a)| {
            let served = a.create.fuse_file_mounts.iter().chain(&a.create.fuse_directory_mounts).any(|m| &m.provider == on && m.volume_name == volume);
            let running = (a.active.is_some() || a.running)
                && a.create.provider.as_ref().is_some_and(|p| &p.identity == on && p.volume_mounts.iter().any(|m| m.volume_name == volume));
            served || running
        })
        .map(|(name, _)| name.clone())
}

fn one_user(volume: &str, other: &str) -> String {
    format!("{volume} keeps its changes, so it has one user at a time, and {other} holds it")
}

/// What a create or an edit may mount: the pinned machine's own volumes,
/// any known machine's over FUSE, a persistent one only if nobody else
/// holds it, and no mount inside another.
fn check_mounts(inner: &Inner, agent: &str, pin: Option<&Identity>, volume_mounts: &[VolumeMount], fuse: &[&FuseMount]) -> Result<(), String> {
    let store = |on: &Identity| inner.providers.iter().find(|p| &p.identity == on).map(|p| p.store.clone());
    match pin {
        Some(on) => {
            let here = store(on).ok_or("the daemon knows no provider by that identity")?;
            for m in volume_mounts {
                if !here.exists(&m.volume_name) {
                    return Err(format!("{} has no volume named \"{}\"", show(on), m.volume_name));
                }
            }
        }
        None if !volume_mounts.is_empty() => return Err("an agent that runs on whichever machine the daemon chooses mounts no volume of one".into()),
        None => {}
    }
    for m in fuse {
        if (m.volume_mode == Mode::Ephemeral) != m.overlay_disk.is_some() {
            return Err(format!("a live mount of {} states how much its changes may take exactly when the volume starts fresh each run", m.volume_name));
        }
        let there = store(&m.provider).ok_or_else(|| format!("the daemon knows no machine {}", show(&m.provider)))?;
        let mode = there.mode(&m.volume_name).ok_or_else(|| format!("{} has no volume named \"{}\"", show(&m.provider), m.volume_name))?;
        if mode == Mode::Persistent {
            if let Some(other) = holder(inner, &m.provider, &m.volume_name, Some(agent)) {
                return Err(one_user(&m.volume_name, &other));
            }
        }
    }
    let paths: Vec<&[String]> = volume_mounts.iter().map(|m| m.container_path.as_slice()).chain(fuse.iter().map(|m| m.container_path.as_slice())).collect();
    for (i, a) in paths.iter().enumerate() {
        if a.is_empty() {
            return Err("a mount needs a place inside the agent; the root is the image's own".into());
        }
        for b in &paths[i + 1..] {
            if a.starts_with(b) || b.starts_with(a) {
                let shorter = if a.len() <= b.len() { a } else { b };
                return Err(format!("two mounts overlap at /{}", shorter.join("/")));
            }
        }
    }
    Ok(())
}

/// A tool the stand-in daemon holds: one it runs, or one it joined.
struct ToolState {
    origin: tools::list::server::response::Origin,
    /// What a created tool mounts; none for a connected one.
    create: Option<tools::create::client::request::Frame>,
    created: DateTime<Utc>,
    /// The agents it's attached to, in the order they were attached.
    agents: Vec<String>,
    last_active: Option<DateTime<Utc>>,
}

struct Inner {
    tools: IndexMap<String, ToolState>,
    agents: IndexMap<String, AgentState>,
    providers: Vec<ProviderState>,
    next_id: u64,
}

#[derive(Clone)]
pub struct StubDaemon {
    inner: Arc<Mutex<Inner>>,
    /// Each machine's folder is under here.
    host_root: PathBuf,
    /// Its own work runs here, so a verb is safe to call from anywhere —
    /// a sync command on the main thread included.
    rt: tokio::runtime::Handle,
    /// The agent door — what an agent's `mcp-call-tool` reaches. Set once
    /// the app has built it; scripts that knock before then get an error.
    door: Arc<OnceLock<Arc<Door>>>,
}

impl StubDaemon {
    /// Must be called inside a tokio runtime; that runtime is the one it keeps.
    pub fn new(host_root: PathBuf) -> Self {
        std::hint::black_box(BUILT_IN);
        let daemon = StubDaemon {
            inner: Arc::new(Mutex::new(Inner { tools: IndexMap::new(), agents: IndexMap::new(), providers: Vec::new(), next_id: 1 })),
            host_root,
            rt: tokio::runtime::Handle::current(),
            door: Arc::new(OnceLock::new()),
        };
        daemon.seed();
        daemon
    }

    /// What each agent was made with. The stand-in's alone: the app records
    /// these as if it had made its seeded agents, which is their story.
    pub fn creates(&self) -> Vec<CreateRequest> {
        self.lock().agents.values().map(|a| a.create.clone()).collect()
    }

    pub fn set_door(&self, door: Arc<Door>) {
        let _ = self.door.set(door);
    }

    /// The agent holding a machine's volume, if any: nothing else may touch it.
    fn held(&self, on: &Identity, volume: &str) -> Option<String> {
        holder(&self.lock(), on, volume, None)
    }

    /// A known machine's disk.
    fn machine(&self, on: &Identity) -> Result<Arc<VolumeStore>, WireError> {
        self.lock()
            .providers
            .iter()
            .find(|p| &p.identity == on)
            .map(|p| p.store.clone())
            .ok_or_else(|| wire_error("the daemon knows no machine by that identity"))
    }

    fn store_for(&self, identity: &Identity) -> Arc<VolumeStore> {
        Arc::new(VolumeStore::new(self.host_root.join("machines").join(slug(identity))))
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, Inner> {
        self.inner.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    /// Scripts read this Mac's workspace: the first machine's.
    fn reader(&self) -> impl Fn(&str) -> String + use<> {
        let store = self.lock().providers.first().map(|p| p.store.clone());
        move |path: &str| store.as_ref().map(|s| s.read_text(path)).unwrap_or_default()
    }

    /// The agent loop for one agent: take the next message, run, repeat;
    /// inactive once nothing waits. `first` is work already under way.
    fn spawn_runner(&self, name: String, first: Vec<Step>) {
        let daemon = self.clone();
        self.rt.spawn(async move { daemon.runner(name, first).await });
    }

    async fn play(&self, name: &str, steps: Vec<Step>, gone: &CancellationToken) -> bool {
        let mut calls = 0u32;
        for step in steps {
            tokio::select! {
                _ = tokio::time::sleep(Duration::from_millis(step.delay_ms)) => {}
                _ = gone.cancelled() => return false,
            }
            match step.piece {
                Piece::Item(item) => {
                    let mut inner = self.lock();
                    let Some(agent) = inner.agents.get_mut(name) else { return false };
                    agent.keep(Utc::now(), item);
                }
                Piece::Door { name: tool, args } => {
                    // The wire's mcp-call-tool: a tool call chunk, the app's
                    // answer through the door, a tool response chunk.
                    calls += 1;
                    let id = format!("door_{}_{}", name, calls);
                    {
                        let mut inner = self.lock();
                        let Some(agent) = inner.agents.get_mut(name) else { return false };
                        agent.keep(Utc::now(), script::chunk(json!({ "type": "assistant_tool_call", "id": id, "name": tool, "arguments": args.to_string() })));
                    }
                    let params = rmcp::model::CallToolRequestParams::new(tool).with_arguments(args.as_object().cloned().unwrap_or_default());
                    let (text, is_error) = match self.door.get() {
                        None => ("the app's door is not open".to_owned(), true),
                        // The run's caller stops waiting if the agent is removed.
                        Some(door) => match door.call(&crate::identity::AgentId::daemon(name), params, gone.clone()).await {
                            Ok(r) => (r.content.iter().filter_map(|c| c.as_text().map(|t| t.text.clone())).collect::<Vec<_>>().join("\n"), r.is_error == Some(true)),
                            Err(e) => (e.message.to_string(), true),
                        },
                    };
                    if gone.is_cancelled() {
                        return false;
                    }
                    let mut inner = self.lock();
                    let Some(agent) = inner.agents.get_mut(name) else { return false };
                    agent.keep(Utc::now(), script::chunk(json!({ "type": "tool_response", "id": id, "content": [{ "type": "text", "text": text }], "isError": is_error })));
                }
            }
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
            // A volume that keeps its changes has one user at a time: a run
            // that would be a second is refused before it starts.
            let conflict = {
                let inner = self.lock();
                inner.agents.get(&name).and_then(|agent| agent.create.provider.as_ref()).and_then(|pin| {
                    let store = inner.providers.iter().find(|p| p.identity == pin.identity)?.store.clone();
                    pin.volume_mounts
                        .iter()
                        .filter(|m| store.mode(&m.volume_name) == Some(Mode::Persistent))
                        .find_map(|m| holder(&inner, &pin.identity, &m.volume_name, Some(&name)).map(|other| one_user(&m.volume_name, &other)))
                })
            };
            if let Some(reason) = conflict {
                let mut inner = self.lock();
                if let Some(agent) = inner.agents.get_mut(&name) {
                    agent.keep(Utc::now(), error_item(reason));
                }
                continue;
            }
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
                None => vec![Step { delay_ms: 400, piece: Piece::Item(error_item("no provider supplies this image")) }],
                Some(kind) => match kind.check(&arguments) {
                    Err(reason) => vec![Step { delay_ms: 600, piece: Piece::Item(error_item(format!("these settings are not a {} agent: {reason}", kind.key()))) }],
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
        let (here_disk, studio_disk) = (self.store_for(&here), self.store_for(&studio));
        here_disk.seed_here();
        studio_disk.seed_studio();
        {
            let mut inner = self.lock();
            inner.providers.push(ProviderState { identity: here.clone(), key: "stand-in".into(), store: here_disk, added: now - TimeDelta::days(9) });
            inner.providers.push(ProviderState { identity: studio.clone(), key: "stand-in".into(), store: studio_disk, added: now - TimeDelta::days(3) });
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
            if let Piece::Item(item) = step.piece {
                notes.keep(t, item);
            }
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
        let pin = Pin { identity: here.clone(), volume_mounts: vec![VolumeMount { volume_name: "workspace".into(), volume_relative_path: vec![], volume_mode: Mode::Persistent, container_path: vec!["workspace".into()] }] };
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

    fn here() -> Identity {
        Identity::Outgoing { address: "127.0.0.1:4640".into() }
    }

    fn studio() -> Identity {
        Identity::IncomingUnbrokered { identity: "studio-pc".into() }
    }

    fn live(on: Identity, volume: &str, to: &str) -> FuseMount {
        FuseMount { provider: on, volume_name: volume.into(), volume_relative_path: vec![], volume_mode: Mode::Persistent, overlay_disk: None, container_path: vec![to.into()] }
    }

    fn edit(name: &str, volume_mounts: Vec<VolumeMount>, dirs: Vec<FuseMount>) -> agents::edit::client::request::Frame {
        agents::edit::client::request::Frame { name: name.into(), volume_mounts, fuse_file_mounts: vec![], fuse_directory_mounts: dirs }
    }

    /// A volume is a machine's own: each lists its own, in its own modes,
    /// and one an agent is running with is held.
    #[tokio::test]
    async fn volumes_are_each_machines_own_and_held_while_used() {
        use volumes::list::server::response::Frame as Listed;
        let root = std::env::temp_dir().join(format!("diverge-desktop-test-v-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let daemon = StubDaemon::new(root);
        let listed = |f: Listed| match f {
            Listed::Volumes(v) => v.into_iter().map(|v| (v.name, v.mode)).collect::<Vec<_>>(),
            Listed::Error(e) => panic!("{e:?}"),
        };
        assert_eq!(
            listed(daemon.volumes_list(&here()).await),
            vec![("datasets".into(), Mode::ReadOnly), ("scratch".into(), Mode::Ephemeral), ("workspace".into(), Mode::Persistent)]
        );
        assert_eq!(listed(daemon.volumes_list(&studio()).await), vec![("media".into(), Mode::Persistent)]);
        assert!(matches!(daemon.volumes_list(&Identity::Outgoing { address: "nowhere:1".into() }).await, Listed::Error(_)));
        // site-fixes is mid-run with this machine's workspace mounted.
        let on = here();
        let stat = |name: &str| daemon.volumes_stat(&on, volumes::stat::client::request::Frame { name: name.into() });
        assert!(matches!(stat("workspace").await, volumes::stat::server::response::Frame::Error(_)));
        assert!(matches!(stat("datasets").await, volumes::stat::server::response::Frame::Stat(_)));
        assert!(matches!(
            daemon.volumes_delete(&here(), volumes::delete::client::request::Frame { name: "workspace".into() }).await,
            volumes::delete::server::response::Frame::Mounted
        ));
        // A mode is the volume's, changed by an edit of it.
        let change = volumes::edit::client::request::Frame { name: "scratch".into(), change: Change::Mode(Mode::ReadOnly) };
        assert!(matches!(daemon.volumes_edit(&here(), change).await, volumes::edit::server::response::Frame::Edited));
        assert_eq!(daemon.machine(&here()).unwrap().mode("scratch"), Some(Mode::ReadOnly));
    }

    /// An edit states the mounts anew, waits for an idle agent, and a volume
    /// that keeps its changes has one user at a time.
    #[tokio::test]
    async fn an_edit_restates_mounts_once_the_agent_is_idle() {
        use agents::edit::server::response::Frame as Edit;
        let root = std::env::temp_dir().join(format!("diverge-desktop-test-e-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let daemon = StubDaemon::new(root);
        assert!(matches!(daemon.agents_edit(edit("nobody", vec![], vec![])).await, Edit::NotFound));
        assert!(matches!(daemon.agents_edit(edit("site-fixes", vec![], vec![])).await, Edit::Active), "site-fixes is working");
        // research-notes runs wherever the daemon chooses: no machine's volumes directly.
        let own = VolumeMount { volume_name: "datasets".into(), volume_relative_path: vec![], volume_mode: Mode::ReadOnly, container_path: vec!["data".into()] };
        assert!(matches!(daemon.agents_edit(edit("research-notes", vec![own], vec![])).await, Edit::Error(_)));
        // Live from the studio PC instead: it holds media from now until it is deleted.
        assert!(matches!(daemon.agents_edit(edit("research-notes", vec![], vec![live(studio(), "media", "media")])).await, Edit::Edited));
        assert_eq!(daemon.held(&studio(), "media").as_deref(), Some("research-notes"));
        // media keeps its changes, so broken-loop cannot have it too...
        assert!(matches!(daemon.agents_edit(edit("broken-loop", vec![], vec![live(studio(), "media", "media")])).await, Edit::Error(_)));
        // ...but a read-only volume has any number of users.
        let data = |who: &str| FuseMount { volume_mode: Mode::ReadOnly, ..live(here(), "datasets", who) };
        assert!(matches!(daemon.agents_edit(edit("broken-loop", vec![], vec![data("data")])).await, Edit::Edited));
        assert!(matches!(daemon.agents_edit(edit("research-notes", vec![], vec![live(studio(), "media", "media"), data("data")])).await, Edit::Edited));
        // Mounts may not stack.
        assert!(matches!(daemon.agents_edit(edit("broken-loop", vec![], vec![data("data"), live(here(), "scratch", "data")])).await, Edit::Error(_)));
        // Stated anew: an empty edit lets media go.
        assert!(matches!(daemon.agents_edit(edit("research-notes", vec![], vec![])).await, Edit::Edited));
        assert_eq!(daemon.held(&studio(), "media"), None);
    }

    /// Tools follow the daemon's rules: attach while an agent works, detach only
    /// once it stops, delete only when attached nowhere, edit only your own.
    #[tokio::test]
    async fn tools_are_created_connected_attached_and_let_go() {
        use futures::StreamExt;
        use tools::list::server::response::{Frame as Listed, Origin};
        let root = std::env::temp_dir().join(format!("diverge-desktop-test-tools-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let daemon = StubDaemon::new(root);
        let image = Image { name: "diverge-desktop-room".into(), digest: UNBUILT_DIGEST.into() };
        let create = |name: &str| tools::create::client::request::Frame { image: image.clone(), memory: 1 << 30, disk: 1 << 30, provider: None, fuse_file_mounts: vec![], fuse_directory_mounts: vec![], arguments: json!({}), name: name.into() };
        assert!(matches!(daemon.tools_create(create("labels")).await, tools::create::server::response::Frame::Created));
        assert!(matches!(daemon.tools_create(create("labels")).await, tools::create::server::response::Frame::InUse));
        let connect = tools::connect::client::request::Frame { provider: studio(), id: "room-7".into(), authorization: "a knock".into(), name: "ada's room".into() };
        assert!(matches!(daemon.tools_connect(connect).await, tools::connect::server::response::Frame::Connected));
        let edit = tools::edit::client::request::Frame { name: "ada's room".into(), volume_mounts: vec![], fuse_file_mounts: vec![], fuse_directory_mounts: vec![] };
        assert!(matches!(daemon.tools_edit(edit).await, tools::edit::server::response::Frame::NotOwned), "not yours to change");
        let attach = |tool: &str, agent: &str| tools::attach::client::request::Frame { tool: tool.into(), agent: agent.into() };
        let detach = |tool: &str, agent: &str| tools::detach::client::request::Frame { tool: tool.into(), agent: agent.into() };
        assert!(matches!(daemon.tools_attach(attach("labels", "site-fixes")).await, tools::attach::server::response::Frame::Attached), "attach while it works");
        assert!(matches!(daemon.tools_detach(detach("labels", "site-fixes")).await, tools::detach::server::response::Frame::Active), "detach only once it stops");
        assert!(matches!(daemon.tools_delete(tools::delete::client::request::Frame { name: "labels".into() }).await, tools::delete::server::response::Frame::Attached));
        assert!(matches!(daemon.tools_attach(attach("labels", "nobody")).await, tools::attach::server::response::Frame::NoAgent));
        let listed: Vec<Listed> = daemon.tools_list(tools::list::client::request::Frame {}).collect().await;
        let labels = listed.iter().find_map(|f| match f { Listed::Tool(t) if t.name == "labels" => Some(t.clone()), _ => None }).unwrap();
        assert!(labels.active && labels.agents == vec!["site-fixes".to_owned()]);
        assert!(listed.iter().any(|f| matches!(f, Listed::Tool(t) if matches!(t.origin, Origin::Connected { .. }))));
        let agents: Vec<agents::list::server::response::Frame> = daemon.agents_list(agents::list::client::request::Frame {}).collect().await;
        assert!(agents.iter().any(|f| matches!(f, agents::list::server::response::Frame::Agent(a) if a.name == "site-fixes" && a.tools == vec!["labels".to_owned()])));
        // An ephemeral volume served live must say how much its changes may take.
        let scratch = FuseMount { provider: here(), volume_name: "scratch".into(), volume_relative_path: vec![], volume_mode: Mode::Ephemeral, overlay_disk: None, container_path: vec!["tmp".into()] };
        assert!(matches!(daemon.agents_edit(edit_frame("research-notes", scratch.clone())).await, agents::edit::server::response::Frame::Error(_)));
        let sized = FuseMount { overlay_disk: Some(1 << 30), ..scratch };
        assert!(matches!(daemon.agents_edit(edit_frame("research-notes", sized)).await, agents::edit::server::response::Frame::Edited));
    }

    fn edit_frame(name: &str, dir: FuseMount) -> agents::edit::client::request::Frame {
        agents::edit::client::request::Frame { name: name.into(), volume_mounts: vec![], fuse_file_mounts: vec![], fuse_directory_mounts: vec![dir] }
    }

    /// An agent reaches through the door: it asks its person, waits, and
    /// acts in a Space as itself once answered.
    #[tokio::test(start_paused = true)]
    async fn an_agent_asks_through_the_door_then_claims_the_task() {
        use crate::spaces::Spaces;
        use futures::StreamExt;
        let root = std::env::temp_dir().join(format!("diverge-desktop-test-door-{}", std::process::id()));
        let daemon = StubDaemon::new(root);
        let identity = Arc::new(crate::identity::Identity::stand_in("juno"));
        let tables = std::env::temp_dir().join(format!("diverge-desktop-test-door-tables-{}", std::process::id()));
        let spaces: Arc<dyn Spaces> = Arc::new(crate::spaces::stub::StubSpaces::new(identity.clone(), tables));
        let door = Arc::new(Door::new(spaces.clone(), identity, Arc::new(daemon.clone()), None));
        daemon.set_door(door.clone());
        let cancel = CancellationToken::new();
        let mut cards = door.watch(cancel.clone());
        let content = vec![serde_json::from_value(json!({ "type": "text", "text": "Claim the open task for me" })).unwrap()];
        let outcome = daemon.agents_message(agents::message::client::request::Frame { name: "research-notes".into(), content }, CancellationToken::new()).await;
        assert!(matches!(outcome, agents::message::server::response::Frame::Delivered));
        // First it asks to read the board, then to claim: two cards, each the whole call.
        let mut asked = Vec::new();
        while asked.len() < 2 {
            let Some(crate::view::CardEvent::Card { card }) = cards.next().await else { continue };
            assert_eq!(card.agent, "research-notes");
            assert_eq!(card.kind, crate::view::CardKind::Choice);
            assert_eq!(card.options, vec![crate::door::YES.to_owned(), crate::door::NO.to_owned()]);
            let call = card.call.clone().expect("a card about a move");
            assert_eq!(call.room, crate::spaces::stub::board());
            asked.push((call.verb.clone(), call.reach, call.about.clone()));
            door.answer(card.id, crate::door::YES.into()).unwrap();
        }
        assert_eq!((asked[0].0.as_str(), asked[0].1), ("space_feed", crate::door::Reach::Read), "reading asks first");
        assert_eq!((asked[1].0.as_str(), asked[1].1), ("claim", crate::door::Reach::Work));
        assert_eq!(asked[1].2.as_deref(), Some("Package the photo resizer as a tool"), "the card names the task");
        // Let the run finish (virtual time).
        for _ in 0..50 {
            tokio::time::sleep(Duration::from_millis(500)).await;
            if !daemon.lock().agents.get("research-notes").map(|a| a.running).unwrap_or(false) {
                break;
            }
        }
        let feed = spaces.read(&crate::spaces::Id { id: crate::spaces::stub::board() }, diverge_desktop_room::room::FEED).await.unwrap();
        let rmcp::model::ResourceContents::TextResourceContents { text, .. } = &feed.contents[0] else { panic!() };
        let moves: Vec<Value> = serde_json::from_str(text).unwrap();
        let open = crate::spaces::stub::open_task();
        let claim = moves.iter().find(|m| m["kind"] == "claim" && m["parent"] == open.as_str()).expect("claimed");
        assert_eq!(claim["author"], "research-notes", "the agent claimed it as itself");
        assert_eq!(claim["agent_of"], "juno", "and the room knows whose agent it is");
        cancel.cancel();
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
        let mut inner = self.lock();
        if inner.agents.contains_key(&request.name) {
            return Frame::InUse;
        }
        let checked = {
            let fuse: Vec<&FuseMount> = request.fuse_file_mounts.iter().chain(&request.fuse_directory_mounts).collect();
            let volume_mounts = request.provider.as_ref().map_or(&[][..], |p| p.volume_mounts.as_slice());
            check_mounts(&inner, &request.name, request.provider.as_ref().map(|p| &p.identity), volume_mounts, &fuse)
        };
        if let Err(reason) = checked {
            return Frame::Error(wire_error(reason));
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
                for t in inner.tools.values_mut() {
                    t.agents.retain(|a| a != &request.name);
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
                    tools: inner.tools.iter().filter(|(_, t)| t.agents.contains(name)).map(|(n, _)| n.clone()).collect(),
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

    async fn agents_edit(&self, request: agents::edit::client::request::Frame) -> agents::edit::server::response::Frame {
        use agents::edit::server::response::Frame;
        let mut inner = self.lock();
        let pin = match inner.agents.get(&request.name) {
            None => return Frame::NotFound,
            Some(agent) if agent.active.is_some() || agent.running => return Frame::Active,
            Some(agent) => agent.create.provider.as_ref().map(|p| p.identity.clone()),
        };
        let checked = {
            let fuse: Vec<&FuseMount> = request.fuse_file_mounts.iter().chain(&request.fuse_directory_mounts).collect();
            check_mounts(&inner, &request.name, pin.as_ref(), &request.volume_mounts, &fuse)
        };
        if let Err(reason) = checked {
            return Frame::Error(wire_error(reason));
        }
        let Some(agent) = inner.agents.get_mut(&request.name) else { return Frame::NotFound };
        if let Some(pin) = agent.create.provider.as_mut() {
            pin.volume_mounts = request.volume_mounts;
        }
        agent.create.fuse_file_mounts = request.fuse_file_mounts;
        agent.create.fuse_directory_mounts = request.fuse_directory_mounts;
        Frame::Edited
    }

    async fn tools_create(&self, request: tools::create::client::request::Frame) -> tools::create::server::response::Frame {
        use tools::create::server::response::Frame;
        let mut inner = self.lock();
        if request.name.trim().is_empty() {
            return Frame::Error(wire_error("a tool needs a name"));
        }
        if inner.tools.contains_key(&request.name) {
            return Frame::InUse;
        }
        let checked = {
            let fuse: Vec<&FuseMount> = request.fuse_file_mounts.iter().chain(&request.fuse_directory_mounts).collect();
            let volume_mounts = request.provider.as_ref().map_or(&[][..], |p| p.volume_mounts.as_slice());
            check_mounts(&inner, &request.name, request.provider.as_ref().map(|p| &p.identity), volume_mounts, &fuse)
        };
        if let Err(reason) = checked {
            return Frame::Error(wire_error(reason));
        }
        let origin = tools::list::server::response::Origin::Created { image: request.image.clone(), provider: request.provider.as_ref().map(|p| Provider { identity: p.identity.clone() }), id: None };
        inner.tools.insert(request.name.clone(), ToolState { origin, create: Some(request), created: Utc::now(), agents: Vec::new(), last_active: None });
        Frame::Created
    }

    async fn tools_edit(&self, request: tools::edit::client::request::Frame) -> tools::edit::server::response::Frame {
        use tools::edit::server::response::Frame;
        let mut inner = self.lock();
        let (pin, active) = match inner.tools.get(&request.name) {
            None => return Frame::NotFound,
            Some(t) if t.create.is_none() => return Frame::NotOwned,
            Some(t) => (
                t.create.as_ref().and_then(|c| c.provider.as_ref().map(|p| p.identity.clone())),
                t.agents.iter().any(|a| inner.agents.get(a).is_some_and(|x| x.active.is_some() || x.running)),
            ),
        };
        if active {
            return Frame::Active;
        }
        let checked = {
            let fuse: Vec<&FuseMount> = request.fuse_file_mounts.iter().chain(&request.fuse_directory_mounts).collect();
            check_mounts(&inner, &request.name, pin.as_ref(), &request.volume_mounts, &fuse)
        };
        if let Err(reason) = checked {
            return Frame::Error(wire_error(reason));
        }
        let Some(create) = inner.tools.get_mut(&request.name).and_then(|t| t.create.as_mut()) else { return Frame::NotFound };
        if let Some(p) = create.provider.as_mut() {
            p.volume_mounts = request.volume_mounts;
        }
        create.fuse_file_mounts = request.fuse_file_mounts;
        create.fuse_directory_mounts = request.fuse_directory_mounts;
        Frame::Edited
    }

    async fn tools_connect(&self, request: tools::connect::client::request::Frame) -> tools::connect::server::response::Frame {
        use tools::connect::server::response::Frame;
        let mut inner = self.lock();
        if inner.tools.contains_key(&request.name) {
            return Frame::InUse;
        }
        if !inner.providers.iter().any(|p| p.identity == request.provider) {
            return Frame::Error(wire_error("the daemon knows no provider by that identity"));
        }
        let origin = tools::list::server::response::Origin::Connected { provider: request.provider, id: request.id };
        inner.tools.insert(request.name, ToolState { origin, create: None, created: Utc::now(), agents: Vec::new(), last_active: None });
        Frame::Connected
    }

    async fn tools_attach(&self, request: tools::attach::client::request::Frame) -> tools::attach::server::response::Frame {
        use tools::attach::server::response::Frame;
        let mut inner = self.lock();
        if !inner.agents.contains_key(&request.agent) {
            return Frame::NoAgent;
        }
        let Some(t) = inner.tools.get_mut(&request.tool) else { return Frame::NoTool };
        if !t.agents.contains(&request.agent) {
            t.agents.push(request.agent);
        }
        Frame::Attached
    }

    async fn tools_detach(&self, request: tools::detach::client::request::Frame) -> tools::detach::server::response::Frame {
        use tools::detach::server::response::Frame;
        let mut inner = self.lock();
        let active = match inner.agents.get(&request.agent) {
            None => return Frame::NoAgent,
            Some(a) => a.active.is_some() || a.running,
        };
        if !inner.tools.contains_key(&request.tool) {
            return Frame::NoTool;
        }
        if active {
            return Frame::Active;
        }
        if let Some(t) = inner.tools.get_mut(&request.tool) {
            t.agents.retain(|a| a != &request.agent);
        }
        Frame::Detached
    }

    async fn tools_delete(&self, request: tools::delete::client::request::Frame) -> tools::delete::server::response::Frame {
        use tools::delete::server::response::Frame;
        let mut inner = self.lock();
        match inner.tools.get(&request.name) {
            None => Frame::NotFound,
            Some(t) if !t.agents.is_empty() => Frame::Attached,
            Some(_) => {
                inner.tools.shift_remove(&request.name);
                Frame::Deleted
            }
        }
    }

    fn tools_list(&self, _request: tools::list::client::request::Frame) -> Frames<tools::list::server::response::Frame> {
        use tools::list::server::response::{Frame, Tool};
        let inner = self.lock();
        let frames: Vec<Frame> = inner
            .tools
            .iter()
            .map(|(name, t)| {
                let active = t.agents.iter().any(|a| inner.agents.get(a).is_some_and(|x| x.active.is_some() || x.running));
                Frame::Tool(Tool { name: name.clone(), origin: t.origin.clone(), created: t.created, active, last_active: t.last_active, agents: t.agents.clone() })
            })
            .collect();
        Box::pin(stream::iter(frames))
    }

    async fn providers_list(&self) -> Vec<ProviderEntry> {
        self.lock()
            .providers
            .iter()
            .map(|p| ProviderEntry { identity: p.identity.clone(), added: p.added })
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
        // A machine's disk outlives the daemon knowing it: add it back and
        // its volumes are there, as on a real machine.
        let entry = ProviderState { store: self.store_for(&identity), identity, key, added: Utc::now() };
        let out = ProviderEntry { identity: entry.identity.clone(), added: entry.added };
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

/// The provider protocol's volume verbs, for every machine the stand-in
/// knows. A real machine enforces its holds itself; here the stand-in does,
/// since it knows every agent's mounts.
#[async_trait]
impl Machines for StubDaemon {
    async fn volumes_list(&self, on: &Identity) -> volumes::list::server::response::Frame {
        use volumes::list::server::response::Frame;
        match self.machine(on) {
            Ok(store) => Frame::Volumes(store.list()),
            Err(error) => Frame::Error(error),
        }
    }

    async fn volumes_stat(&self, on: &Identity, request: volumes::stat::client::request::Frame) -> volumes::stat::server::response::Frame {
        use volumes::stat::server::response::Frame;
        let store = match self.machine(on) {
            Ok(store) => store,
            Err(error) => return Frame::Error(error),
        };
        if let Some(agent) = self.held(on, &request.name) {
            return Frame::Error(wire_error(format!("{agent} has it mounted")));
        }
        match store.stat(&request.name) {
            Ok(stat) => Frame::Stat(stat),
            Err(reason) => Frame::Error(wire_error(reason)),
        }
    }

    fn volumes_read(&self, on: &Identity, request: volumes::read::client::request::Frame) -> Frames<ReadFrame> {
        let frames = match self.machine(on) {
            Err(error) => vec![ReadFrame::Error(error)],
            Ok(store) => match self.held(on, &request.name) {
                Some(agent) => vec![ReadFrame::Error(wire_error(format!("{agent} has it mounted")))],
                None => match store.read(&request.name, &request.path) {
                    Err(reason) => vec![ReadFrame::Error(wire_error(reason))],
                    Ok(bytes) if bytes.is_empty() => Vec::new(),
                    Ok(bytes) => bytes.chunks(64 * 1024).map(|piece| ReadFrame::Body(piece.to_vec())).collect(),
                },
            },
        };
        Box::pin(stream::iter(frames))
    }

    async fn volumes_write(&self, on: &Identity, request: volumes::write::client::request::Frame, body: Vec<u8>) -> volumes::write::server::response::Frame {
        use volumes::write::server::response::Frame;
        let store = match self.machine(on) {
            Ok(store) => store,
            Err(error) => return Frame::Error(error),
        };
        if let Some(agent) = self.held(on, &request.name) {
            return Frame::Error(wire_error(format!("{agent} has it mounted")));
        }
        match store.write(&request.name, &request.path, &body) {
            Ok(()) => Frame::Written(write_path::response::Frame),
            Err(reason) => Frame::Error(wire_error(reason)),
        }
    }

    async fn volumes_filetree(&self, on: &Identity, request: volumes::filetree::client::request::Frame) -> volumes::filetree::server::response::Frame {
        use volumes::filetree::server::response::Frame;
        let store = match self.machine(on) {
            Ok(store) => store,
            Err(error) => return Frame::Error(error),
        };
        if let Some(agent) = self.held(on, &request.name) {
            return Frame::Error(wire_error(format!("{agent} has it mounted")));
        }
        match store.tree(&request.name, &request.path) {
            Ok(tree) => Frame::Tree(tree),
            Err(reason) => Frame::Error(wire_error(reason)),
        }
    }

    async fn volumes_create_capacity(&self, on: &Identity) -> volumes::create_capacity::server::response::Frame {
        use volumes::create_capacity::server::response::Frame;
        match self.machine(on) {
            Ok(store) => Frame::Capacity(store.create_capacity()),
            Err(error) => Frame::Error(error),
        }
    }

    async fn volumes_create(&self, on: &Identity, request: volumes::create::client::request::Frame) -> volumes::create::server::response::Frame {
        use volumes::create::server::response::Frame;
        let store = match self.machine(on) {
            Ok(store) => store,
            Err(error) => return Frame::Error(error),
        };
        match store.create(&request.name, request.bytes, request.mode) {
            Ok(true) => Frame::Created,
            Ok(false) => Frame::InsufficientCapacity,
            Err(reason) => Frame::Error(wire_error(reason)),
        }
    }

    async fn volumes_edit_capacity(&self, on: &Identity, request: volumes::edit_capacity::client::request::Frame) -> volumes::edit_capacity::server::response::Frame {
        use volumes::edit_capacity::server::response::Frame;
        let store = match self.machine(on) {
            Ok(store) => store,
            Err(error) => return Frame::Error(error),
        };
        match store.edit_capacity(&request.name) {
            Ok(bytes) => Frame::Capacity(bytes),
            Err(reason) => Frame::Error(wire_error(reason)),
        }
    }

    async fn volumes_edit(&self, on: &Identity, request: volumes::edit::client::request::Frame) -> volumes::edit::server::response::Frame {
        use volumes::edit::server::response::Frame;
        let store = match self.machine(on) {
            Ok(store) => store,
            Err(error) => return Frame::Error(error),
        };
        if let Some(agent) = self.held(on, &request.name) {
            return Frame::Error(wire_error(format!("{agent} has it mounted")));
        }
        let (bytes, mode) = match request.change {
            Change::Bytes(bytes) => (Some(bytes), None),
            Change::Mode(mode) => (None, Some(mode)),
            Change::Both { bytes, mode } => (Some(bytes), Some(mode)),
        };
        match store.edit(&request.name, bytes, mode) {
            Ok(None) => Frame::Edited,
            Ok(Some("room")) => Frame::InsufficientCapacity,
            Ok(Some(_)) => Frame::ContentTooLarge,
            Err(reason) => Frame::Error(wire_error(reason)),
        }
    }

    async fn volumes_delete(&self, on: &Identity, request: volumes::delete::client::request::Frame) -> volumes::delete::server::response::Frame {
        use volumes::delete::server::response::Frame;
        let store = match self.machine(on) {
            Ok(store) => store,
            Err(error) => return Frame::Error(error),
        };
        if self.held(on, &request.name).is_some() {
            return Frame::Mounted;
        }
        match store.delete(&request.name) {
            Ok(()) => Frame::Deleted,
            Err(reason) => Frame::Error(wire_error(reason)),
        }
    }
}
