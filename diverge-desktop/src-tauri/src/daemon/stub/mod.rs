//! The stand-in daemon: the daemon's verbs, answered locally, with the SDK's types.
//!
//! It keeps templates, agents and their logs in memory, runs scripted
//! "runs" (see [`script`]), and answers for every machine it knows as well:
//! each machine's volumes are real folders (see [`store`]), behind the
//! other seam, [`crate::machines`]. It follows the SDK's documented
//! semantics — a template held by its hash, so the same one made twice is
//! one; agents numbered per template; an agents list that says `Listed`,
//! stays open and says what changes (an agent made, changed or deleted: a run
//! starting or ending, an edit, its tags, its tools); spans inclusive, the type before the count, a watch that ends only
//! when nothing more can match, a delete or an edit of the mounts that
//! leaves an active agent alone, a message that can be taken back only
//! until it is delivered, a volume nothing touches while anything holds
//! it, and a volume that keeps its changes with one user at a time.

pub mod script;
pub mod store;

use std::collections::{BTreeSet, HashMap, VecDeque};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use async_trait::async_trait;
use chrono::{DateTime, TimeDelta, Utc};
use futures::{Stream, StreamExt, stream};
use indexmap::IndexMap;
use rmcp::model::ContentBlock;
use serde_json::{Value, json};
use tokio::sync::{broadcast, mpsc, oneshot};
use tokio_util::sync::CancellationToken;

use diverge_sdk::daemon::create::Inner as Made;
use diverge_sdk::daemon::creator::{self, Creator};
use diverge_sdk::daemon::edit::{Change as EditChange, Edit};
use diverge_sdk::daemon::endpoints::agents::create::client::request::{
    Frame as CreateRequest, FuseMount, Provider as Pin, VolumeMount,
};
use diverge_sdk::daemon::endpoints::agents::logs::client::request::{Frame as LogsRequest, ItemType};
use diverge_sdk::daemon::endpoints::agents::logs::server::response::{
    Active, ActiveType, Error as ErrorItem, ErrorType, Identity, Inactive, InactiveType, Item, ItemWrapper, Provider, User,
};
use diverge_sdk::daemon::endpoints::agents::templates::{AgentType, Template as AgentTemplate};
use diverge_sdk::daemon::endpoints::standalone_tools::templates::Template as ToolTemplate;
use diverge_sdk::daemon::endpoints::{agents, standalone_tools};
use diverge_sdk::daemon::{key, reference};
use diverge_sdk::provider::endpoints::containers::agents::run::server::response::AgenticLoopChunk;
use diverge_sdk::provider::endpoints::volumes;
use diverge_sdk::provider::endpoints::volumes::Mode;
use diverge_sdk::provider::endpoints::volumes::edit::client::request::Change;
use diverge_sdk::shared::containers::write_path;
use diverge_sdk::shared::error::Error as WireError;

use super::{Daemon, Frames, NewProvider, ProviderEntry, agent_key, template_id};
use crate::machines::{Machines, ReadFrame};
use crate::catalog::Kind;
use script::{Piece, Step};
use store::VolumeStore;
use crate::door::Door;
use std::sync::OnceLock;

/// Present in any build the stand-in is part of: the check that it's absent looks for these words.
pub const BUILT_IN: &str = "diverge-desktop stand-in daemon is built in";

/// How long a message waits before the agent takes it, when nothing is
/// ahead of it: long enough to take it back.
const DELIVERY: Duration = Duration::from_millis(2500);

/// The account the app is served as: the stand-in's word for its one client.
const CLIENT: &str = "stand-in";

fn client() -> Creator {
    Creator::Client(creator::Client { identity: CLIENT.into() })
}

fn wire_error(message: impl Into<String>) -> WireError {
    WireError(json!({ "message": message.into() }))
}

/// An agent's id inside the stand-in: its template and its index, which
/// name it once and for all, whatever it is called.
fn id_of(template: &str, index: u64) -> String {
    format!("{template}#{index}")
}

struct Queued {
    id: u64,
    content: Vec<ContentBlock>,
    delivered: oneshot::Sender<()>,
}

struct AgentState {
    create: CreateRequest,
    index: u64,
    creator: Creator,
    /// From its template: which of the six images, and its settings.
    kind: Option<Kind>,
    arguments: Value,
    created: DateTime<Utc>,
    log: Vec<ItemWrapper>,
    active: Option<Provider>,
    queue: VecDeque<Queued>,
    running: bool,
    live: broadcast::Sender<ItemWrapper>,
    gone: CancellationToken,
    /// What `tag` put on it and `untag` hasn't taken off: a set, sorted bytewise.
    tags: BTreeSet<String>,
}

impl AgentState {
    fn new(create: CreateRequest, index: u64, creator: Creator, template: &AgentTemplate, created: DateTime<Utc>) -> Self {
        let (live, _) = broadcast::channel(1024);
        AgentState {
            create,
            index,
            creator,
            kind: Kind::of_image(&template.image),
            arguments: template.arguments.clone(),
            created,
            log: Vec::new(),
            active: None,
            queue: VecDeque::new(),
            running: false,
            live,
            gone: CancellationToken::new(),
            tags: BTreeSet::new(),
        }
    }

    /// Keep an item; indexes count up from 1, so `0` is an empty log.
    fn keep(&mut self, created: DateTime<Utc>, item: Item) {
        let created = self.log.last().map_or(created, |last| created.max(last.created));
        let wrapper = ItemWrapper { logs_index: self.log.len() as u64 + 1, created, item };
        self.log.push(wrapper.clone());
        let _ = self.live.send(wrapper);
    }

    /// How the app keys it, and how its words name it.
    fn key(&self) -> String {
        agent_key(self.create.name.as_deref(), &self.create.inner.template, self.index)
    }

    fn busy(&self) -> bool {
        self.active.is_some() || self.running
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

/// How an agent holds a machine's volume: served over FUSE, or mounted in
/// its running container.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Hold {
    Served,
    Mounted,
}

/// Who holds a machine's volume, if anyone, by the app's key, and how. An
/// agent that mounts it over FUSE holds it for its life: the daemon's serve
/// runs from the create to the delete. An agent pinned there with it
/// mounted holds it while a run is under way. Nothing examines, reads,
/// writes, walks, resizes or deletes a volume while anything holds it.
fn holder(inner: &Inner, on: &Identity, volume: &str, except: Option<&str>) -> Option<(String, Hold)> {
    inner.agents.iter().filter(|(id, _)| Some(id.as_str()) != except).find_map(|(_, a)| {
        let made = &a.create.inner;
        if made.fuse_file_mounts.iter().chain(&made.fuse_directory_mounts).any(|m| &m.provider == on && m.volume_name == volume) {
            return Some((a.key(), Hold::Served));
        }
        let running = a.busy() && made.provider.as_ref().is_some_and(|p| &p.identity == on && p.volume_mounts.iter().any(|m| m.volume_name == volume));
        running.then(|| (a.key(), Hold::Mounted))
    })
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
            if let Some((other, _)) = holder(inner, &m.provider, &m.volume_name, Some(agent)) {
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

/// An edit's member: as it is, taken away, or replaced whole.
fn changed<T: Clone + Default>(now: &T, change: &Option<EditChange<T>>) -> T {
    match change {
        None => now.clone(),
        Some(EditChange::Delete) => T::default(),
        Some(EditChange::Set(value)) => value.clone(),
    }
}

/// A template the stand-in holds, by its id.
struct Held<T> {
    template: T,
    created: DateTime<Utc>,
    creator: Creator,
}

/// A tool the stand-in daemon holds: one made from a template, or another
/// daemon's, registered.
struct ToolState {
    name: Option<String>,
    origin: standalone_tools::list::server::response::Origin,
    /// What a created tool mounts; none for a registered one.
    create: Option<standalone_tools::create::client::request::Frame>,
    creator: Creator,
    created: DateTime<Utc>,
    /// The agents it's attached to, by id, in the order they were attached.
    agents: Vec<String>,
    /// The agent templates it's attached to, likewise.
    templates: Vec<String>,
    last_active: Option<DateTime<Utc>>,
}

impl ToolState {
    /// How an agent's list item names it.
    fn key(&self) -> key::Tool {
        use standalone_tools::list::server::response::Origin;
        let (origin, index) = match &self.origin {
            Origin::Created { template, index, mode: _, provider: _, id: _ } => (key::Origin::Created { template: template.clone() }, *index),
            Origin::Connected { daemon, account, tool, index } => {
                (key::Origin::Connected { daemon: daemon.clone(), account: account.clone(), tool: Box::new(tool.clone()) }, *index)
            }
        };
        key::Tool::Record { origin, index, name: self.name.clone() }
    }

    fn is(&self, tool: &reference::Tool) -> bool {
        use standalone_tools::list::server::response::Origin;
        match (tool, &self.origin) {
            (reference::Tool::Name { name }, _) => self.name.as_ref() == Some(name),
            (reference::Tool::TemplateIndex { template, index }, Origin::Created { template: t, index: i, .. }) => template == t && index == i,
            (reference::Tool::Connected { daemon, account, tool }, Origin::Connected { daemon: d, account: a, tool: t, .. }) => daemon == d && account == a && **tool == *t,
            (reference::Tool::TemplateIndex { .. }, Origin::Connected { .. }) | (reference::Tool::Connected { .. }, Origin::Created { .. }) => false,
        }
    }
}

struct Inner {
    templates: IndexMap<String, Held<AgentTemplate>>,
    tool_templates: IndexMap<String, Held<ToolTemplate>>,
    /// The last number given under each template (and each registered tool).
    counters: HashMap<String, u64>,
    tools: Vec<ToolState>,
    agents: IndexMap<String, AgentState>,
    providers: Vec<ProviderState>,
    next_id: u64,
    /// A tick each time something an agents list reports may have changed:
    /// an agent made or deleted, a run starting or ending, an edit, its tags
    /// or its tools. Every open list reads the agents again and says what differs.
    changes: broadcast::Sender<()>,
}

impl Inner {
    /// Something a list reports may have changed: every open list looks again.
    fn changed(&self) {
        let _ = self.changes.send(());
    }

    /// The next number under a template: the first is 1, and none is given twice.
    fn next_index(&mut self, of: &str) -> u64 {
        let n = self.counters.entry(of.to_owned()).or_insert(0);
        *n += 1;
        *n
    }

    fn find(&self, agent: &reference::Agent) -> Option<String> {
        match agent {
            reference::Agent::Name { name } => self.agents.iter().find(|(_, a)| a.create.name.as_deref() == Some(name)).map(|(id, _)| id.clone()),
            reference::Agent::TemplateIndex { template, index } => Some(id_of(template, *index)).filter(|id| self.agents.contains_key(id)),
        }
    }

    fn find_tool(&self, tool: &reference::Tool) -> Option<usize> {
        self.tools.iter().position(|t| t.is(tool))
    }

    /// The tools an agent has: attached to it, then to its template, once each.
    fn tools_of(&self, id: &str, template: &str) -> Vec<key::Tool> {
        let direct = self.tools.iter().filter(|t| t.agents.iter().any(|a| a == id));
        let by_template = self.tools.iter().filter(|t| t.templates.iter().any(|x| x == template) && !t.agents.iter().any(|a| a == id));
        direct.chain(by_template).map(ToolState::key).collect()
    }

    fn tool_active(&self, t: &ToolState) -> bool {
        self.agents.iter().any(|(id, a)| a.busy() && (t.agents.contains(id) || t.templates.contains(&a.create.inner.template)))
    }

    /// The agent as a list or a get answers it.
    fn listed(&self, id: &str, agent: &AgentState) -> agents::list::server::response::Agent {
        // Its activity and its log's length as of its last change of state:
        // a line appended changes nothing a list reports.
        let last_change = agent.log.iter().rev().find_map(|item| match &item.item {
            Item::Active(Active { provider, .. }) | Item::Inactive(Inactive { provider, .. }) => Some((item.created, provider.clone(), item.logs_index)),
            Item::User(_) | Item::Chunk(_) | Item::Error(_) => None,
        });
        agents::list::server::response::Agent {
            name: agent.create.name.clone(),
            template: agent.create.inner.template.clone(),
            index: agent.index,
            creator: agent.creator.clone(),
            created: agent.created,
            active: agent.active.is_some(),
            last_active: last_change.as_ref().map(|(at, _, _)| *at),
            logs_index: last_change.as_ref().map_or(0, |(_, _, index)| *index),
            provider: last_change.map(|(_, provider, _)| provider),
            tools: self.tools_of(id, &agent.create.inner.template),
            tags: agent.tags.iter().cloned().collect(),
        }
    }

    /// What a list asks for, as the stand-in holds it now: every agent that
    /// matches, oldest made first, the first `count` of them, by id.
    fn window(&self, request: &agents::list::client::request::Frame) -> IndexMap<String, agents::list::server::response::Agent> {
        let mut all: Vec<(&String, &AgentState)> = self.agents.iter().collect();
        all.sort_by_key(|(_, a)| a.created);
        all.into_iter()
            .map(|(id, agent)| (id.clone(), self.listed(id, agent)))
            .filter(|(_, agent)| matches(&request.filter, agent))
            .take(request.count.map_or(usize::MAX, |n| n as usize))
            .collect()
    }
}

/// A list stays open after `Listed`, until its scope is cancelled. The
/// stand-in's lists say nothing more in that time.
fn open_until<T: Send + 'static>(cancel: CancellationToken) -> impl Stream<Item = T> + Send + 'static {
    stream::once(cancel.cancelled_owned()).filter_map(|()| futures::future::ready(None))
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
        let inner = Inner {
            templates: IndexMap::new(),
            tool_templates: IndexMap::new(),
            counters: HashMap::new(),
            tools: Vec::new(),
            agents: IndexMap::new(),
            providers: Vec::new(),
            next_id: 1,
            changes: broadcast::channel(64).0,
        };
        let daemon = StubDaemon { inner: Arc::new(Mutex::new(inner)), host_root, rt: tokio::runtime::Handle::current(), door: Arc::new(OnceLock::new()) };
        daemon.seed();
        daemon
    }

    /// What each agent the app made was made with, by the app's key. The
    /// stand-in's alone: the app records these as if it had made its seeded
    /// agents, which is their story; one an agent made is not the app's.
    pub fn creates(&self) -> Vec<(String, CreateRequest)> {
        self.lock().agents.values().filter(|a| a.creator == client()).map(|a| (a.key(), a.create.clone())).collect()
    }

    pub fn set_door(&self, door: Arc<Door>) {
        let _ = self.door.set(door);
    }

    /// The agent holding a machine's volume, if any: nothing else may touch it.
    fn held(&self, on: &Identity, volume: &str) -> Option<(String, Hold)> {
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
    fn spawn_runner(&self, id: String, first: Vec<Step>) {
        let daemon = self.clone();
        self.rt.spawn(async move { daemon.runner(id, first).await });
    }

    async fn play(&self, id: &str, steps: Vec<Step>, gone: &CancellationToken) -> bool {
        let mut calls = 0u32;
        for step in steps {
            tokio::select! {
                _ = tokio::time::sleep(Duration::from_millis(step.delay_ms)) => {}
                _ = gone.cancelled() => return false,
            }
            match step.piece {
                Piece::Item(item) => {
                    let mut inner = self.lock();
                    let Some(agent) = inner.agents.get_mut(id) else { return false };
                    agent.keep(Utc::now(), item);
                }
                Piece::Door { name: tool, args } => {
                    // The wire's mcp-call-tool: a tool call chunk, the app's
                    // answer through the door, a tool response chunk.
                    calls += 1;
                    let key = {
                        let mut inner = self.lock();
                        let Some(agent) = inner.agents.get_mut(id) else { return false };
                        let key = agent.key();
                        agent.keep(Utc::now(), script::chunk(json!({ "type": "assistant_tool_call", "id": format!("door_{key}_{calls}"), "name": tool, "arguments": args.to_string() })));
                        key
                    };
                    let call = format!("door_{key}_{calls}");
                    let params = rmcp::model::CallToolRequestParams::new(tool).with_arguments(args.as_object().cloned().unwrap_or_default());
                    let (text, is_error) = match self.door.get() {
                        None => ("the app's door is not open".to_owned(), true),
                        // The run's caller stops waiting if the agent is removed.
                        Some(door) => match door.call(&crate::identity::AgentId::daemon(&key), params, gone.clone()).await {
                            Ok(r) => (r.content.iter().filter_map(|c| c.as_text().map(|t| t.text.clone())).collect::<Vec<_>>().join("\n"), r.is_error == Some(true)),
                            Err(e) => (e.message.to_string(), true),
                        },
                    };
                    if gone.is_cancelled() {
                        return false;
                    }
                    let mut inner = self.lock();
                    let Some(agent) = inner.agents.get_mut(id) else { return false };
                    agent.keep(Utc::now(), script::chunk(json!({ "type": "tool_response", "id": call, "content": [{ "type": "text", "text": text }], "isError": is_error })));
                }
            }
        }
        true
    }

    async fn runner(&self, id: String, first: Vec<Step>) {
        let Some(gone) = self.lock().agents.get(&id).map(|a| a.gone.clone()) else { return };
        if !first.is_empty() && !self.play(&id, first, &gone).await {
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
                let Some(agent) = inner.agents.get_mut(&id) else { return };
                let Some(message) = agent.queue.pop_front() else {
                    if let Some(provider) = agent.active.take() {
                        agent.keep(Utc::now(), Item::Inactive(Inactive { r#type: InactiveType::Inactive, provider }));
                    }
                    agent.running = false;
                    inner.changed();
                    return;
                };
                let provider = agent.create.inner.provider.as_ref().map(|p| p.identity.clone()).or(first_provider);
                (message, agent.kind, agent.arguments.clone(), provider)
            };
            let _ = message.delivered.send(());
            let text = keep_user_parts(self, &id, message.id, &message.content, client());

            let Some(identity) = provider else {
                let mut inner = self.lock();
                if let Some(agent) = inner.agents.get_mut(&id) {
                    agent.keep(Utc::now(), error_item("no provider to run on: add a machine first"));
                }
                continue;
            };
            // A volume that keeps its changes has one user at a time: a run
            // that would be a second is refused before it starts.
            let conflict = {
                let inner = self.lock();
                inner.agents.get(&id).and_then(|agent| agent.create.inner.provider.as_ref()).and_then(|pin| {
                    let store = inner.providers.iter().find(|p| p.identity == pin.identity)?.store.clone();
                    pin.volume_mounts
                        .iter()
                        .filter(|m| store.mode(&m.volume_name) == Some(Mode::Persistent))
                        .find_map(|m| holder(&inner, &pin.identity, &m.volume_name, Some(&id)).map(|(other, _)| one_user(&m.volume_name, &other)))
                })
            };
            if let Some(reason) = conflict {
                let mut inner = self.lock();
                if let Some(agent) = inner.agents.get_mut(&id) {
                    agent.keep(Utc::now(), error_item(reason));
                }
                continue;
            }
            {
                let mut inner = self.lock();
                let Some(agent) = inner.agents.get_mut(&id) else { return };
                if agent.active.is_none() {
                    let provider = Provider { identity };
                    agent.keep(Utc::now(), Item::Active(Active { r#type: ActiveType::Active, provider: provider.clone() }));
                    agent.active = Some(provider);
                    inner.changed();
                }
            }
            let steps = match kind {
                None => vec![Step { delay_ms: 400, piece: Piece::Item(error_item("no provider supplies this image")) }],
                Some(kind) => match kind.check(&arguments) {
                    Err(reason) => vec![Step { delay_ms: 600, piece: Piece::Item(error_item(format!("these settings are not a {} agent: {reason}", kind.key()))) }],
                    Ok(()) => script::run(kind, &text, &self.reader()),
                },
            };
            if !self.play(&id, steps, &gone).await {
                return;
            }
        }
    }

    /// Hold a template as `templates::create` would, and number a new agent
    /// made from it: its id, and the agent's index and state.
    fn make(&self, template: AgentTemplate, name: Option<&str>, provider: Option<Pin>, creator: Creator, created: DateTime<Utc>) -> (String, AgentState) {
        let mut inner = self.lock();
        let id = template_id(&template);
        let index = inner.next_index(&id);
        let held = inner.templates.entry(id.clone()).or_insert(Held { template, created, creator: client() });
        let create = CreateRequest { inner: Made { template: id.clone(), provider, fuse_file_mounts: Vec::new(), fuse_directory_mounts: Vec::new() }, name: name.map(Into::into), relay: None };
        let agent = AgentState::new(create, index, creator, &held.template, created);
        (id_of(&id, index), agent)
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
        let template = |kind: Kind, arguments: Value| AgentTemplate { r#type: AgentType {}, description: None, image: kind.image(), memory: 4 << 30, disk: 8 << 30, arguments };
        let read = self.reader();

        // A finished run from yesterday.
        let (notes_id, mut notes) = self.make(template(Kind::Hermes, json!({ "provider": { "provider": "openrouter", "api_key": "stand-in" }, "model": "nousresearch/hermes-4-70b" })), Some("research-notes"), None, client(), now - TimeDelta::days(2));
        let mut t = now - TimeDelta::hours(20);
        notes.keep(t, user_text(1, "Pull the open questions out of my notes", client()));
        notes.keep(t, Item::Active(Active { r#type: ActiveType::Active, provider: Provider { identity: here.clone() } }));
        for step in script::run(Kind::Hermes, "Pull the open questions out of my notes", &read) {
            t += TimeDelta::milliseconds(step.delay_ms as i64);
            if let Piece::Item(item) = step.piece {
                notes.keep(t, item);
            }
        }
        notes.keep(t + TimeDelta::seconds(1), Item::Inactive(Inactive { r#type: InactiveType::Inactive, provider: Provider { identity: here.clone() } }));

        // A run that failed.
        let (broken_id, mut broken) = self.make(template(Kind::Openrouter, json!({ "model": "openrouter/auto" })), Some("broken-loop"), None, client(), now - TimeDelta::hours(6));
        let t = now - TimeDelta::hours(5);
        broken.keep(t, user_text(2, "Summarise the reading list", client()));
        broken.keep(t, Item::Active(Active { r#type: ActiveType::Active, provider: Provider { identity: studio.clone() } }));
        broken.keep(t + TimeDelta::seconds(2), script::chunk(json!({ "type": "assistant_reasoning", "text": "Reading the list first." })));
        broken.keep(t + TimeDelta::seconds(4), error_item("the upstream refused the request: no API key for it in the vault"));
        broken.keep(t + TimeDelta::seconds(4), Item::Inactive(Inactive { r#type: InactiveType::Inactive, provider: Provider { identity: studio.clone() } }));

        // One mid-way through a long job, pinned to this machine's workspace.
        let pin = Pin { identity: here.clone(), volume_mounts: vec![VolumeMount { volume_name: "workspace".into(), volume_relative_path: vec![], volume_mode: Mode::Persistent, container_path: vec!["workspace".into()] }] };
        let (site_id, mut site) = self.make(template(Kind::Cc, json!({ "model": "claude-sonnet-5", "tools": { "read": true, "glob": true, "grep": true, "bash": true, "task": true } })), Some("site-fixes"), Some(pin), client(), now - TimeDelta::minutes(40));
        let t = now - TimeDelta::seconds(20);
        site.keep(t, user_text(3, "Go through the whole site and tidy it, section by section.", client()));
        site.keep(t, Item::Active(Active { r#type: ActiveType::Active, provider: Provider { identity: here.clone() } }));
        site.active = Some(Provider { identity: here.clone() });
        site.running = true;

        // One site-fixes made through the daemon, given no name, and set to work.
        let maker = Creator::Agent(creator::Agent { template: site.create.inner.template.clone(), index: site.index, name: site.create.name.clone() });
        let ask = "Check every link on the site's front page and list the broken ones.";
        let (links_id, mut links) = self.make(template(Kind::Openrouter, json!({ "model": "openrouter/auto", "system_prompt": "Check links. Say what's broken; change nothing." })), None, None, maker.clone(), now - TimeDelta::minutes(30));
        let mut t = now - TimeDelta::minutes(29);
        links.keep(t, user_text(4, ask, maker));
        links.keep(t, Item::Active(Active { r#type: ActiveType::Active, provider: Provider { identity: here.clone() } }));
        for step in script::run(Kind::Openrouter, ask, &read) {
            t += TimeDelta::milliseconds(step.delay_ms as i64);
            if let Piece::Item(item) = step.piece {
                links.keep(t, item);
            }
        }
        links.keep(t + TimeDelta::seconds(1), Item::Inactive(Inactive { r#type: InactiveType::Inactive, provider: Provider { identity: here } }));

        {
            let mut inner = self.lock();
            inner.next_id = 5;
            inner.agents.insert(site_id.clone(), site);
            inner.agents.insert(notes_id, notes);
            inner.agents.insert(broken_id, broken);
            inner.agents.insert(links_id, links);
        }
        self.spawn_runner(site_id, script::long_job(&read));
    }
}

fn user_text(id: u64, text: &str, sender: Creator) -> Item {
    let Item::Chunk(chunk) = script::chunk(json!({ "type": "user_text_content", "key": format!("m{id}"), "text": text })) else { unreachable!("a user text part is a chunk") };
    Item::User(User { sender, chunk })
}

fn error_item(message: impl Into<String>) -> Item {
    Item::Error(ErrorItem { r#type: ErrorType::Error, error: wire_error(message) })
}

/// Keep a delivered message's parts as user items, with who sent it; the text, for the run.
fn keep_user_parts(daemon: &StubDaemon, id: &str, message: u64, content: &[ContentBlock], sender: Creator) -> String {
    let mut text = String::new();
    let mut inner = daemon.lock();
    let Some(agent) = inner.agents.get_mut(id) else { return text };
    for block in content {
        let value = serde_json::to_value(block).unwrap_or(Value::Null);
        let kind = value.get("type").and_then(Value::as_str).unwrap_or_default().to_owned();
        let mut part = value.clone();
        if let Some(object) = part.as_object_mut() {
            object.remove("type");
            object.insert("key".into(), json!(format!("m{message}")));
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
            agent.keep(Utc::now(), Item::User(User { sender: sender.clone(), chunk }));
        }
    }
    text
}

#[cfg(test)]
mod tests {
    use super::*;

    fn by_name(name: &str) -> reference::Agent {
        reference::Agent::Name { name: name.into() }
    }

    fn logs(agent: reference::Agent, count: Option<u64>) -> LogsRequest {
        LogsRequest { agent, logs_index_from: None, logs_index_to: None, created_from: None, created_to: None, r#type: None, count, watch: None, relay: None }
    }

    /// A sync Tauri command runs on the main thread, outside any runtime.
    /// Draft one crashed here once; never again.
    #[test]
    fn verbs_work_from_a_thread_with_no_runtime() {
        use futures::StreamExt;
        let rt = tokio::runtime::Runtime::new().unwrap();
        let root = std::env::temp_dir().join(format!("diverge-desktop-test-rt-{}", std::process::id()));
        let daemon = rt.block_on(async { StubDaemon::new(root) });
        let frames = std::thread::spawn(move || daemon.agents_logs(logs(by_name("research-notes"), Some(3)), CancellationToken::new())).join().unwrap();
        let got = rt.block_on(frames.collect::<Vec<_>>());
        assert_eq!(got.len(), 3);
    }

    /// A template is held by its hash: the same one twice is one template,
    /// and each agent made from it gets the next number.
    #[tokio::test]
    async fn templates_are_held_by_hash_and_agents_numbered_per_template() {
        use agents::templates::create::server::response::Frame as Answer;
        let root = std::env::temp_dir().join(format!("diverge-desktop-test-tpl-{}", std::process::id()));
        let daemon = StubDaemon::new(root);
        let template = AgentTemplate { r#type: AgentType {}, description: None, image: Kind::Openrouter.image(), memory: 1 << 30, disk: 1 << 30, arguments: json!({ "model": "openrouter/auto", "system_prompt": "Be brief." }) };
        let make = |template: AgentTemplate| agents::templates::create::client::request::Frame { template, relay: None };
        let Answer::Created(id) = daemon.agents_templates_create(make(template.clone())).await else { panic!("made") };
        assert_eq!(id, template_id(&template));
        // The references are not the template: another name for one image is the same one.
        let renamed = AgentTemplate { image: agents::create::client::request::Image { references: Vec::new(), ..template.image.clone() }, ..template.clone() };
        assert!(matches!(daemon.agents_templates_create(make(renamed)).await, Answer::Exists(again) if again == id));
        let create = |name: Option<&str>| CreateRequest { inner: made(id.clone()), name: name.map(Into::into), relay: None };
        use agents::create::server::response::Frame as Created;
        assert!(matches!(daemon.agents_create(create(None)).await, Created::Created));
        assert!(matches!(daemon.agents_create(create(Some("brief"))).await, Created::Created));
        assert!(matches!(daemon.agents_create(create(Some("brief"))).await, Created::InUse));
        let listed = crate::daemon::agents_listed(&daemon).await;
        assert!(matches!(listed.last(), Some(agents::list::server::response::Frame::Listed)), "a list says it has listed");
        let mine: Vec<_> = listed
            .iter()
            .filter_map(|f| match f {
                agents::list::server::response::Frame::Added(a) if a.template == id => Some((a.name.clone(), a.index, a.creator.clone())),
                _ => None,
            })
            .collect();
        assert_eq!(mine, vec![(None, 1, client()), (Some("brief".to_owned()), 2, client())]);
        // The one with no name is reached by its template and index.
        let unnamed = reference::Agent::TemplateIndex { template: id.clone(), index: 1 };
        assert!(matches!(daemon.agents_delete(agents::delete::client::request::Frame { agent: unnamed, relay: None }).await, agents::delete::server::response::Frame::Deleted));
    }

    fn made(template: String) -> Made {
        Made { template, provider: None, fuse_file_mounts: Vec::new(), fuse_directory_mounts: Vec::new() }
    }

    /// The seeded agent another agent made: no name, its maker named, and
    /// what its maker said to it marked as its maker's.
    #[tokio::test]
    async fn an_agent_made_by_an_agent_says_who_made_it() {
        use futures::StreamExt;
        let root = std::env::temp_dir().join(format!("diverge-desktop-test-made-{}", std::process::id()));
        let daemon = StubDaemon::new(root);
        let listed = crate::daemon::agents_listed(&daemon).await;
        let agents: Vec<_> = listed.into_iter().filter_map(|f| match f { agents::list::server::response::Frame::Added(a) => Some(a), _ => None }).collect();
        let site = agents.iter().find(|a| a.name.as_deref() == Some("site-fixes")).unwrap();
        let made = agents.iter().find(|a| a.name.is_none()).expect("one agent with no name");
        assert_eq!(made.creator, Creator::Agent(creator::Agent { template: site.template.clone(), index: site.index, name: Some("site-fixes".into()) }));
        let reference = crate::daemon::reference_of(&agent_key(None, &made.template, made.index));
        let mut items = daemon.agents_logs(LogsRequest { r#type: Some(ItemType::UserTextContent), ..logs(reference, None) }, CancellationToken::new());
        let Some(agents::logs::server::response::Frame::Item(first)) = items.next().await else { panic!("a user part") };
        assert!(matches!(first.item, Item::User(User { sender, .. }) if sender == made.creator));
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
        let edit = Edit { name: None, volume_mounts: Some(EditChange::Set(volume_mounts)), fuse_file_mounts: Some(EditChange::Set(vec![])), fuse_directory_mounts: Some(EditChange::Set(dirs)) };
        agents::edit::client::request::Frame { agent: by_name(name), edit, relay: None }
    }

    /// A volume is a machine's own: each lists its own, in its own modes,
    /// and one an agent is running with is held.
    #[tokio::test]
    async fn volumes_are_each_machines_own_and_held_while_used() {
        use volumes::list::server::response::Frame as Listed;
        let root = std::env::temp_dir().join(format!("diverge-desktop-test-v-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let daemon = StubDaemon::new(root);
        let listed = async |on: Identity| {
            let cancel = CancellationToken::new();
            let mut out = Vec::new();
            let mut frames = daemon.volumes_list(&on, cancel.clone());
            while let Some(frame) = frames.next().await {
                match frame {
                    Listed::Added(v) => out.push((v.name, v.mode)),
                    Listed::Listed => break,
                    other => panic!("{other:?}"),
                }
            }
            cancel.cancel();
            out
        };
        assert_eq!(listed(here()).await, vec![("datasets".into(), Mode::ReadOnly), ("scratch".into(), Mode::Ephemeral), ("workspace".into(), Mode::Persistent)]);
        assert_eq!(listed(studio()).await, vec![("media".into(), Mode::Persistent)]);
        let nowhere: Vec<Listed> = daemon.volumes_list(&Identity::Outgoing { address: "nowhere:1".into() }, CancellationToken::new()).take(1).collect().await;
        assert!(matches!(nowhere[..], [Listed::Error(_)]));
        // site-fixes is mid-run with this machine's workspace mounted.
        let on = here();
        let stat = |name: &str| daemon.volumes_stat(&on, volumes::stat::client::request::Frame { name: name.into() });
        assert!(matches!(stat("workspace").await, volumes::stat::server::response::Frame::Mounted));
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
        use agents::edit::server::response::Frame as Edited;
        let root = std::env::temp_dir().join(format!("diverge-desktop-test-e-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let daemon = StubDaemon::new(root);
        assert!(matches!(daemon.agents_edit(edit("nobody", vec![], vec![])).await, Edited::NotFound));
        assert!(matches!(daemon.agents_edit(edit("site-fixes", vec![], vec![])).await, Edited::Active), "site-fixes is working");
        // research-notes runs wherever the daemon chooses: no machine's volumes directly.
        let own = VolumeMount { volume_name: "datasets".into(), volume_relative_path: vec![], volume_mode: Mode::ReadOnly, container_path: vec!["data".into()] };
        assert!(matches!(daemon.agents_edit(edit("research-notes", vec![own], vec![])).await, Edited::Error(_)));
        // Live from the studio PC instead: it holds media from now until it is deleted.
        assert!(matches!(daemon.agents_edit(edit("research-notes", vec![], vec![live(studio(), "media", "media")])).await, Edited::Edited));
        assert_eq!(daemon.held(&studio(), "media"), Some(("research-notes".to_owned(), Hold::Served)));
        // media keeps its changes, so broken-loop cannot have it too...
        assert!(matches!(daemon.agents_edit(edit("broken-loop", vec![], vec![live(studio(), "media", "media")])).await, Edited::Error(_)));
        // ...but a read-only volume has any number of users.
        let data = |who: &str| FuseMount { volume_mode: Mode::ReadOnly, ..live(here(), "datasets", who) };
        assert!(matches!(daemon.agents_edit(edit("broken-loop", vec![], vec![data("data")])).await, Edited::Edited));
        assert!(matches!(daemon.agents_edit(edit("research-notes", vec![], vec![live(studio(), "media", "media"), data("data")])).await, Edited::Edited));
        // Mounts may not stack.
        assert!(matches!(daemon.agents_edit(edit("broken-loop", vec![], vec![data("data"), live(here(), "scratch", "data")])).await, Edited::Error(_)));
        // Stated anew: an empty edit lets media go.
        assert!(matches!(daemon.agents_edit(edit("research-notes", vec![], vec![])).await, Edited::Edited));
        assert_eq!(daemon.held(&studio(), "media"), None);
        // A name changes while it works; another's name is taken.
        let rename = |agent: &str, to: &str| agents::edit::client::request::Frame { agent: by_name(agent), edit: Edit { name: Some(EditChange::Set(to.into())), ..Edit::default() }, relay: None };
        assert!(matches!(daemon.agents_edit(rename("site-fixes", "broken-loop")).await, Edited::InUse));
        assert!(matches!(daemon.agents_edit(rename("site-fixes", "site-tidy")).await, Edited::Edited));
    }

    /// Tools follow the daemon's rules: attached to an agent or its
    /// template, deleted only when attached nowhere, edited only if yours.
    #[tokio::test]
    async fn tools_are_created_registered_attached_and_let_go() {
        use futures::StreamExt;
        use standalone_tools::list::server::response::{Frame as Listed, Origin};
        let root = std::env::temp_dir().join(format!("diverge-desktop-test-tools-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let daemon = StubDaemon::new(root);
        let template = ToolTemplate { r#type: Default::default(), description: None, image: Kind::Python.image(), memory: 1 << 30, disk: 1 << 30, arguments: json!({}) };
        let standalone_tools::templates::create::server::response::Frame::Created(tid) =
            daemon.standalone_tools_templates_create(standalone_tools::templates::create::client::request::Frame { template, relay: None }).await
        else {
            panic!("made")
        };
        let create = |name: &str| standalone_tools::create::client::request::Frame { inner: made(tid.clone()), account: None, name: Some(name.into()), relay: None };
        assert!(matches!(daemon.standalone_tools_create(create("labels")).await, standalone_tools::create::server::response::Frame::Created));
        assert!(matches!(daemon.standalone_tools_create(create("labels")).await, standalone_tools::create::server::response::Frame::InUse));
        let tool = |name: &str| reference::Tool::Name { name: name.into() };
        let register = standalone_tools::register::client::request::Frame { daemon: "studio".into(), account: "guest".into(), tool: tool("rooms"), name: Some("a friend's room".into()), relay: None };
        assert!(matches!(daemon.standalone_tools_register(register).await, standalone_tools::register::server::response::Frame::Registered));
        let theirs = standalone_tools::edit::client::request::Frame { tool: tool("a friend's room"), edit: Edit::default(), account: None, relay: None };
        assert!(matches!(daemon.standalone_tools_edit(theirs).await, standalone_tools::edit::server::response::Frame::NotOwned), "not yours to change");
        let attach = |t: &str, agent: &str| standalone_tools::attach_agent_instance::client::request::Frame { tool: tool(t), agent: by_name(agent), relay: None };
        assert!(matches!(daemon.standalone_tools_attach_agent_instance(attach("labels", "site-fixes")).await, standalone_tools::attach_agent_instance::server::response::Frame::Attached));
        assert!(matches!(daemon.standalone_tools_attach_agent_instance(attach("labels", "nobody")).await, standalone_tools::attach_agent_instance::server::response::Frame::NoAgent));
        assert!(matches!(
            daemon.standalone_tools_delete(standalone_tools::delete::client::request::Frame { tool: tool("labels"), relay: None }).await,
            standalone_tools::delete::server::response::Frame::Attached
        ));
        let cancel = CancellationToken::new();
        let listed: Vec<Listed> = daemon.standalone_tools_list(standalone_tools::list::client::request::Frame { filter: Default::default(), count: None, relay: None }, cancel.clone()).take(3).collect().await;
        cancel.cancel();
        assert!(matches!(listed.last(), Some(Listed::Listed)));
        let labels = listed.iter().find_map(|f| match f { Listed::Added(t) if t.name.as_deref() == Some("labels") => Some(t.clone()), _ => None }).unwrap();
        assert!(labels.active && labels.attached_agent_instances.iter().map(|a| a.name.as_deref()).eq([Some("site-fixes")]));
        assert!(listed.iter().any(|f| matches!(f, Listed::Added(t) if matches!(t.origin, Origin::Connected { .. }))));
        let site = crate::daemon::agents_listed(&daemon)
            .await
            .into_iter()
            .find_map(|f| match f { agents::list::server::response::Frame::Added(a) if a.name.as_deref() == Some("site-fixes") => Some(a), _ => None })
            .unwrap();
        assert!(matches!(&site.tools[..], [key::Tool::Record { name: Some(n), .. }] if n == "labels"));
        // Attached to a template, every agent made from it has the tool.
        let detach = standalone_tools::detach_agent_instance::client::request::Frame { tool: tool("labels"), agent: by_name("site-fixes"), relay: None };
        assert!(matches!(daemon.standalone_tools_detach_agent_instance(detach).await, standalone_tools::detach_agent_instance::server::response::Frame::Detached));
        let to_template = standalone_tools::attach_agent_template::client::request::Frame { tool: tool("labels"), template: site.template.clone(), relay: None };
        assert!(matches!(daemon.standalone_tools_attach_agent_template(to_template).await, standalone_tools::attach_agent_template::server::response::Frame::Attached));
        let get = daemon.agents_templates_get(agents::templates::get::client::request::Frame { id: site.template.clone(), relay: None }).await;
        assert!(matches!(get, agents::templates::get::server::response::Frame::Found(t) if t.tools.len() == 1));
        // An ephemeral volume served live must say how much its changes may take.
        let scratch = FuseMount { provider: here(), volume_name: "scratch".into(), volume_relative_path: vec![], volume_mode: Mode::Ephemeral, overlay_disk: None, container_path: vec!["tmp".into()] };
        assert!(matches!(daemon.agents_edit(edit("research-notes", vec![], vec![scratch.clone()])).await, agents::edit::server::response::Frame::Error(_)));
        let sized = FuseMount { overlay_disk: Some(1 << 30), ..scratch };
        assert!(matches!(daemon.agents_edit(edit("research-notes", vec![], vec![sized])).await, agents::edit::server::response::Frame::Edited));
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
        let outcome = daemon.agents_message(agents::message::client::request::Frame { agent: by_name("research-notes"), content, relay: None }, CancellationToken::new()).await;
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
        let id = daemon.lock().find(&by_name("research-notes")).unwrap();
        for _ in 0..50 {
            tokio::time::sleep(Duration::from_millis(500)).await;
            if !daemon.lock().agents.get(&id).map(|a| a.running).unwrap_or(false) {
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

    /// Read a list on until it has said `n` more frames, or nothing comes for a while.
    async fn next_frames(frames: &mut Frames<agents::list::server::response::Frame>, n: usize) -> Vec<agents::list::server::response::Frame> {
        let mut out = Vec::new();
        while out.len() < n {
            match tokio::time::timeout(Duration::from_secs(5), frames.next()).await {
                Ok(Some(frame)) => out.push(frame),
                Ok(None) | Err(_) => break,
            }
        }
        out
    }

    /// The list stays open past `Listed` and says what changes: an agent
    /// made, its tags, its tools, a run starting, and its removal, as it was
    /// last sent. A line its run writes changes nothing a list says.
    #[tokio::test(start_paused = true)]
    async fn the_agents_list_says_what_changes() {
        use agents::list::server::response::Frame as Listed;
        let root = std::env::temp_dir().join(format!("diverge-desktop-test-live-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let daemon = StubDaemon::new(root);
        let cancel = CancellationToken::new();
        let mut frames = daemon.agents_list(agents::list::client::request::Frame::default(), cancel.clone());
        let first = next_frames(&mut frames, 64).await;
        assert!(matches!(first.last(), Some(Listed::Listed)), "{first:?}");
        let listed: Vec<_> = first.iter().filter_map(|f| match f { Listed::Added(a) => Some(a), _ => None }).collect();
        assert!(listed.windows(2).all(|w| w[0].created <= w[1].created), "oldest made first");
        assert_eq!(listed[0].name.as_deref(), Some("research-notes"));
        // site-fixes is mid-run: its lines land, and the list says nothing.
        assert!(next_frames(&mut frames, 1).await.is_empty(), "no frame per log line");

        // Made: added.
        let template = AgentTemplate { r#type: AgentType {}, description: None, image: Kind::Openrouter.image(), memory: 1 << 30, disk: 1 << 30, arguments: json!({ "model": "openrouter/auto" }) };
        let agents::templates::create::server::response::Frame::Created(id) = daemon.agents_templates_create(agents::templates::create::client::request::Frame { template, relay: None }).await else { panic!("made") };
        daemon.agents_create(CreateRequest { inner: made(id), name: Some("brief".into()), relay: None }).await;
        assert!(matches!(&next_frames(&mut frames, 1).await[..], [Listed::Added(a)] if a.name.as_deref() == Some("brief") && !a.active));

        // Tagged, then a tool attached: changed, as it is now.
        let tag = |tags: &[&str]| agents::tag::client::request::Frame { agent: by_name("brief"), tags: tags.iter().map(|t| t.to_string()).collect(), relay: None };
        assert!(matches!(daemon.agents_tag(tag(&["weekly", "notes"])).await, agents::tag::server::response::Frame::Tagged));
        assert!(matches!(&next_frames(&mut frames, 1).await[..], [Listed::Changed(a)] if a.tags == ["notes", "weekly"]), "sorted");
        let tools = ToolTemplate { r#type: Default::default(), description: None, image: Kind::Python.image(), memory: 1 << 30, disk: 1 << 30, arguments: json!({}) };
        let standalone_tools::templates::create::server::response::Frame::Created(tid) =
            daemon.standalone_tools_templates_create(standalone_tools::templates::create::client::request::Frame { template: tools, relay: None }).await
        else {
            panic!("made")
        };
        daemon.standalone_tools_create(standalone_tools::create::client::request::Frame { inner: made(tid), account: None, name: Some("labels".into()), relay: None }).await;
        assert!(next_frames(&mut frames, 1).await.is_empty(), "a tool attached to nothing changes no agent");
        let tool = reference::Tool::Name { name: "labels".into() };
        let attach = standalone_tools::attach_agent_instance::client::request::Frame { tool, agent: by_name("brief"), relay: None };
        assert!(matches!(daemon.standalone_tools_attach_agent_instance(attach).await, standalone_tools::attach_agent_instance::server::response::Frame::Attached));
        assert!(matches!(&next_frames(&mut frames, 1).await[..], [Listed::Changed(a)] if a.tools.len() == 1));

        // A run starting: changed, active, its log's length as of the start.
        let content = vec![serde_json::from_value(json!({ "type": "text", "text": "Say hello" })).unwrap()];
        let message = agents::message::client::request::Frame { agent: by_name("brief"), content, relay: None };
        assert!(matches!(daemon.agents_message(message, CancellationToken::new()).await, agents::message::server::response::Frame::Delivered));
        let started = next_frames(&mut frames, 1).await;
        let [Listed::Changed(running)] = &started[..] else { panic!("{started:?}") };
        assert!(running.active && running.logs_index > 0);
        // …and ending: changed again, no longer active.
        let ended = next_frames(&mut frames, 1).await;
        assert!(matches!(&ended[..], [Listed::Changed(a)] if !a.active && a.logs_index > running.logs_index), "{ended:?}");

        // Removed: as it was last sent.
        assert!(matches!(daemon.agents_untag(agents::untag::client::request::Frame { agent: by_name("brief"), tags: vec!["weekly".into(), "never-on-it".into()], relay: None }).await, agents::untag::server::response::Frame::Untagged));
        assert!(matches!(&next_frames(&mut frames, 1).await[..], [Listed::Changed(a)] if a.tags == ["notes"]));
        assert!(matches!(daemon.agents_delete(agents::delete::client::request::Frame { agent: by_name("brief"), relay: None }).await, agents::delete::server::response::Frame::Deleted));
        assert!(matches!(&next_frames(&mut frames, 1).await[..], [Listed::Removed(a)] if a.name.as_deref() == Some("brief") && a.tags == ["notes"]));
        cancel.cancel();
        assert!(next_frames(&mut frames, 1).await.is_empty(), "cancelled, it finishes");
    }

    /// A list asked with a filter says an agent come to match as added and
    /// one that matches no more as removed.
    #[tokio::test]
    async fn a_filtered_list_adds_and_removes_as_agents_come_and_go() {
        use agents::list::server::response::Frame as Listed;
        let root = std::env::temp_dir().join(format!("diverge-desktop-test-live-filter-{}", std::process::id()));
        let daemon = StubDaemon::new(root);
        let cancel = CancellationToken::new();
        let filter = agents::list::client::request::Filter { any_tags: vec!["urgent".into()], ..Default::default() };
        let mut frames = daemon.agents_list(agents::list::client::request::Frame { filter, count: None, relay: None }, cancel.clone());
        assert!(matches!(&next_frames(&mut frames, 1).await[..], [Listed::Listed]), "none match yet");
        daemon.agents_tag(agents::tag::client::request::Frame { agent: by_name("broken-loop"), tags: vec!["urgent".into()], relay: None }).await;
        assert!(matches!(&next_frames(&mut frames, 1).await[..], [Listed::Added(a)] if a.name.as_deref() == Some("broken-loop")));
        daemon.agents_untag(agents::untag::client::request::Frame { agent: by_name("broken-loop"), tags: vec!["urgent".into()], relay: None }).await;
        assert!(matches!(&next_frames(&mut frames, 1).await[..], [Listed::Removed(a)] if a.tags == ["urgent"]), "as it was last sent");
        assert!(matches!(daemon.agents_tag(agents::tag::client::request::Frame { agent: by_name("nobody"), tags: vec![], relay: None }).await, agents::tag::server::response::Frame::NotFound));
        cancel.cancel();
    }

    #[tokio::test]
    async fn seeded_agents_have_settings_their_image_accepts() {
        let root = std::env::temp_dir().join(format!("diverge-desktop-test-{}", std::process::id()));
        let daemon = StubDaemon::new(root);
        let inner = daemon.lock();
        for agent in inner.agents.values() {
            let kind = agent.kind.expect("a seeded agent names one of the six images");
            assert!(kind.check(&agent.arguments).is_ok(), "{}: {:?}", agent.key(), kind.check(&agent.arguments));
        }
    }
}

/// The `type` a chunk carries, as a logs request names one.
fn chunk_type(chunk: &AgenticLoopChunk) -> ItemType {
    match chunk {
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
    }
}

/// The `type` an item carries, as a logs request names one.
fn item_type(item: &Item) -> ItemType {
    match item {
        Item::User(user) => chunk_type(&user.chunk),
        Item::Chunk(chunk) => chunk_type(chunk),
        Item::Error(_) => ItemType::Error,
        Item::Active(_) => ItemType::Active,
        Item::Inactive(_) => ItemType::Inactive,
    }
}

/// The spans and the type: what the read sends.
fn in_filter(request: &LogsRequest, item: &ItemWrapper) -> bool {
    request.logs_index_from.is_none_or(|from| item.logs_index >= from)
        && request.logs_index_to.is_none_or(|to| item.logs_index <= to)
        && request.created_from.is_none_or(|from| item.created >= from)
        && request.created_to.is_none_or(|to| item.created <= to)
        && request.r#type.is_none_or(|kind| item_type(&item.item) == kind)
}

/// Whether a list item matches what a list asks for.
fn matches(filter: &agents::list::client::request::Filter, agent: &agents::list::server::response::Agent) -> bool {
    (filter.names.is_empty() || agent.name.as_ref().is_some_and(|n| filter.names.contains(n)))
        && (filter.templates.is_empty() || filter.templates.contains(&agent.template))
        && (filter.creators.is_empty() || filter.creators.contains(&agent.creator))
        && filter.active.is_none_or(|active| active == agent.active)
        && filter.all_tags.iter().all(|t| agent.tags.contains(t))
        && (filter.any_tags.is_empty() || filter.any_tags.iter().any(|t| agent.tags.contains(t)))
        && filter.created_from.is_none_or(|from| agent.created >= from)
        && filter.created_to.is_none_or(|to| agent.created <= to)
}

#[async_trait]
impl Daemon for StubDaemon {
    async fn agents_templates_create(
        &self,
        request: agents::templates::create::client::request::Frame,
    ) -> agents::templates::create::server::response::Frame {
        use agents::templates::create::server::response::Frame;
        let id = template_id(&request.template);
        let mut inner = self.lock();
        if inner.templates.contains_key(&id) {
            return Frame::Exists(id);
        }
        inner.templates.insert(id.clone(), Held { template: request.template, created: Utc::now(), creator: client() });
        Frame::Created(id)
    }

    async fn agents_templates_get(&self, request: agents::templates::get::client::request::Frame) -> agents::templates::get::server::response::Frame {
        use agents::templates::get::server::response::Frame;
        use agents::templates::list::server::response::Listed;
        let inner = self.lock();
        match inner.templates.get(&request.id) {
            None => Frame::NotFound,
            Some(held) => Frame::Found(Listed {
                id: request.id.clone(),
                created: held.created,
                creator: held.creator.clone(),
                tags: Vec::new(),
                tools: inner.tools.iter().filter(|t| t.templates.contains(&request.id)).map(ToolState::key).collect(),
                template: held.template.clone(),
            }),
        }
    }

    async fn agents_create(&self, request: CreateRequest) -> agents::create::server::response::Frame {
        use agents::create::server::response::Frame;
        if request.name.as_ref().is_some_and(|n| n.trim().is_empty()) {
            return Frame::Error(wire_error("a name, if given, says something"));
        }
        let mut inner = self.lock();
        let Some(held) = inner.templates.get(&request.inner.template) else {
            return Frame::Error(wire_error("no template of yours has that id"));
        };
        let template = held.template.clone();
        if request.name.is_some() && inner.agents.values().any(|a| a.create.name == request.name) {
            return Frame::InUse;
        }
        let index = inner.counters.get(&request.inner.template).copied().unwrap_or(0) + 1;
        let id = id_of(&request.inner.template, index);
        let checked = {
            let made = &request.inner;
            let fuse: Vec<&FuseMount> = made.fuse_file_mounts.iter().chain(&made.fuse_directory_mounts).collect();
            let volume_mounts = made.provider.as_ref().map_or(&[][..], |p| p.volume_mounts.as_slice());
            check_mounts(&inner, &id, made.provider.as_ref().map(|p| &p.identity), volume_mounts, &fuse)
        };
        if let Err(reason) = checked {
            return Frame::Error(wire_error(reason));
        }
        inner.next_index(&request.inner.template);
        inner.agents.insert(id, AgentState::new(request, index, client(), &template, Utc::now()));
        inner.changed();
        Frame::Created
    }

    async fn agents_delete(&self, request: agents::delete::client::request::Frame) -> agents::delete::server::response::Frame {
        use agents::delete::server::response::Frame;
        let mut inner = self.lock();
        let Some(id) = inner.find(&request.agent) else { return Frame::NotFound };
        if inner.agents.get(&id).is_some_and(AgentState::busy) {
            return Frame::Active;
        }
        if let Some(agent) = inner.agents.shift_remove(&id) {
            agent.gone.cancel();
        }
        for t in &mut inner.tools {
            t.agents.retain(|a| a != &id);
        }
        inner.changed();
        Frame::Deleted
    }

    async fn agents_message(
        &self,
        request: agents::message::client::request::Frame,
        cancel: CancellationToken,
    ) -> agents::message::server::response::Frame {
        use agents::message::server::response::Frame;
        let (delivered, mut on_delivery) = oneshot::channel();
        let (id, message, start) = {
            let mut inner = self.lock();
            let message = inner.next_id;
            inner.next_id += 1;
            let Some(id) = inner.find(&request.agent) else {
                return Frame::Error(wire_error("no agent of yours is the one named"));
            };
            let Some(agent) = inner.agents.get_mut(&id) else { return Frame::Error(wire_error("no agent of yours is the one named")) };
            agent.queue.push_back(Queued { id: message, content: request.content, delivered });
            let start = !agent.running;
            agent.running = true;
            (id, message, start)
        };
        if start {
            self.spawn_runner(id.clone(), Vec::new());
        }
        tokio::select! {
            result = &mut on_delivery => match result {
                Ok(()) => Frame::Delivered,
                Err(_) => Frame::Error(wire_error("the agent was deleted")),
            },
            _ = cancel.cancelled() => {
                {
                    let mut inner = self.lock();
                    if let Some(agent) = inner.agents.get_mut(&id) {
                        if let Some(at) = agent.queue.iter().position(|q| q.id == message) {
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
                inner.find(&request.agent).and_then(|id| inner.agents.get(&id)).map(|agent| (agent.log.clone(), agent.live.subscribe(), agent.gone.clone()))
            };
            let Some((history, mut live, gone)) = found else {
                let _ = tx.send(Frame::Error(wire_error("no agent of yours is the one named"))).await;
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
                if tx.send(Frame::Item(item.clone())).await.is_err() {
                    return false;
                }
                *sent += 1;
                !limit.is_some_and(|limit| *sent >= limit)
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

    /// Every agent that matches, `Listed`, then what changes, until `cancel`:
    /// on each tick the window is read again, and an agent come into it is
    /// `Added`, one in it still that differs from what was last sent
    /// `Changed`, and one gone from it `Removed`, as it was last sent.
    fn agents_list(&self, request: agents::list::client::request::Frame, cancel: CancellationToken) -> Frames<agents::list::server::response::Frame> {
        use agents::list::server::response::Frame;
        let (tx, rx) = mpsc::channel::<Frame>(256);
        // The window and the subscription under one lock: no change falls between them.
        let (mut sent, mut changes) = {
            let inner = self.lock();
            (inner.window(&request), inner.changes.subscribe())
        };
        let daemon = self.clone();
        self.rt.spawn(async move {
            let first = sent.values().cloned().map(Frame::Added).chain([Frame::Listed]);
            for frame in first.collect::<Vec<_>>() {
                if tx.send(frame).await.is_err() {
                    return;
                }
            }
            loop {
                tokio::select! {
                    _ = cancel.cancelled() => return,
                    _ = tx.closed() => return,
                    next = changes.recv() => match next {
                        // A list that fell behind reads the window again all the same.
                        Ok(()) | Err(broadcast::error::RecvError::Lagged(_)) => {}
                        Err(broadcast::error::RecvError::Closed) => return,
                    },
                }
                let now = daemon.lock().window(&request);
                let gone = sent.iter().filter(|(id, _)| !now.contains_key(*id)).map(|(_, was)| Frame::Removed(was.clone()));
                let came = now.iter().filter_map(|(id, agent)| match sent.get(id) {
                    None => Some(Frame::Added(agent.clone())),
                    Some(was) if was != agent => Some(Frame::Changed(agent.clone())),
                    Some(_) => None,
                });
                let frames: Vec<Frame> = gone.chain(came).collect();
                sent = now;
                for frame in frames {
                    if tx.send(frame).await.is_err() {
                        return;
                    }
                }
            }
        });
        Box::pin(stream::unfold(rx, |mut rx| async move { rx.recv().await.map(|frame| (frame, rx)) }))
    }

    async fn agents_edit(&self, request: agents::edit::client::request::Frame) -> agents::edit::server::response::Frame {
        use agents::edit::server::response::Frame;
        let mut inner = self.lock();
        let Some(id) = inner.find(&request.agent) else { return Frame::NotFound };
        let Some(agent) = inner.agents.get(&id) else { return Frame::NotFound };
        let Edit { name, volume_mounts, fuse_file_mounts, fuse_directory_mounts } = &request.edit;
        let mounts = volume_mounts.is_some() || fuse_file_mounts.is_some() || fuse_directory_mounts.is_some();
        if mounts && agent.busy() {
            return Frame::Active;
        }
        let new_name = match name {
            None => agent.create.name.clone(),
            Some(EditChange::Delete) => None,
            Some(EditChange::Set(n)) => Some(n.clone()),
        };
        if new_name.is_some() && inner.agents.iter().any(|(other, a)| other != &id && a.create.name == new_name) {
            return Frame::InUse;
        }
        let made = &agent.create.inner;
        let pin = made.provider.as_ref().map(|p| p.identity.clone());
        let volume_mounts = changed(&made.provider.as_ref().map(|p| p.volume_mounts.clone()).unwrap_or_default(), volume_mounts);
        let fuse_file_mounts = changed(&made.fuse_file_mounts, fuse_file_mounts);
        let fuse_directory_mounts = changed(&made.fuse_directory_mounts, fuse_directory_mounts);
        let checked = {
            let fuse: Vec<&FuseMount> = fuse_file_mounts.iter().chain(&fuse_directory_mounts).collect();
            check_mounts(&inner, &id, pin.as_ref(), &volume_mounts, &fuse)
        };
        if let Err(reason) = checked {
            return Frame::Error(wire_error(reason));
        }
        let Some(agent) = inner.agents.get_mut(&id) else { return Frame::NotFound };
        if let Some(pin) = agent.create.inner.provider.as_mut() {
            pin.volume_mounts = volume_mounts;
        }
        agent.create.inner.fuse_file_mounts = fuse_file_mounts;
        agent.create.inner.fuse_directory_mounts = fuse_directory_mounts;
        agent.create.name = new_name;
        inner.changed();
        Frame::Edited
    }

    async fn agents_tag(&self, request: agents::tag::client::request::Frame) -> agents::tag::server::response::Frame {
        use agents::tag::server::response::Frame;
        let mut inner = self.lock();
        let Some(id) = inner.find(&request.agent) else { return Frame::NotFound };
        let Some(agent) = inner.agents.get_mut(&id) else { return Frame::NotFound };
        agent.tags.extend(request.tags);
        inner.changed();
        Frame::Tagged
    }

    async fn agents_untag(&self, request: agents::untag::client::request::Frame) -> agents::untag::server::response::Frame {
        use agents::untag::server::response::Frame;
        let mut inner = self.lock();
        let Some(id) = inner.find(&request.agent) else { return Frame::NotFound };
        let Some(agent) = inner.agents.get_mut(&id) else { return Frame::NotFound };
        for tag in &request.tags {
            agent.tags.remove(tag);
        }
        inner.changed();
        Frame::Untagged
    }

    async fn standalone_tools_templates_create(
        &self,
        request: standalone_tools::templates::create::client::request::Frame,
    ) -> standalone_tools::templates::create::server::response::Frame {
        use standalone_tools::templates::create::server::response::Frame;
        let id = template_id(&request.template);
        let mut inner = self.lock();
        if inner.tool_templates.contains_key(&id) {
            return Frame::Exists(id);
        }
        inner.tool_templates.insert(id.clone(), Held { template: request.template, created: Utc::now(), creator: client() });
        Frame::Created(id)
    }

    async fn standalone_tools_create(&self, request: standalone_tools::create::client::request::Frame) -> standalone_tools::create::server::response::Frame {
        use standalone_tools::create::server::response::Frame;
        use standalone_tools::list::server::response::Origin;
        let mut inner = self.lock();
        if request.name.as_ref().is_some_and(|n| n.trim().is_empty()) {
            return Frame::Error(wire_error("a name, if given, says something"));
        }
        let Some(mode) = inner.tool_templates.get(&request.inner.template).map(|held| held.template.r#type.mode) else {
            return Frame::Error(wire_error("no tool template of yours has that id"));
        };
        if request.name.is_some() && inner.tools.iter().any(|t| t.name == request.name) {
            return Frame::InUse;
        }
        // The stand-in holds no accounts on anyone's behalf.
        if request.account.is_some() {
            return Frame::NoAccount;
        }
        let checked = {
            let made = &request.inner;
            let fuse: Vec<&FuseMount> = made.fuse_file_mounts.iter().chain(&made.fuse_directory_mounts).collect();
            let volume_mounts = made.provider.as_ref().map_or(&[][..], |p| p.volume_mounts.as_slice());
            check_mounts(&inner, "", made.provider.as_ref().map(|p| &p.identity), volume_mounts, &fuse)
        };
        if let Err(reason) = checked {
            return Frame::Error(wire_error(reason));
        }
        let index = inner.next_index(&request.inner.template);
        let origin = Origin::Created { template: request.inner.template.clone(), index, mode, provider: None, id: None };
        inner.tools.push(ToolState { name: request.name.clone(), origin, create: Some(request), creator: client(), created: Utc::now(), agents: Vec::new(), templates: Vec::new(), last_active: None });
        Frame::Created
    }

    async fn standalone_tools_edit(&self, request: standalone_tools::edit::client::request::Frame) -> standalone_tools::edit::server::response::Frame {
        use standalone_tools::edit::server::response::Frame;
        let mut inner = self.lock();
        let Some(at) = inner.find_tool(&request.tool) else { return Frame::NotFound };
        let Some(create) = inner.tools[at].create.clone() else { return Frame::NotOwned };
        let Edit { name, volume_mounts, fuse_file_mounts, fuse_directory_mounts } = &request.edit;
        let mounts = volume_mounts.is_some() || fuse_file_mounts.is_some() || fuse_directory_mounts.is_some();
        if mounts && inner.tool_active(&inner.tools[at]) {
            return Frame::Active;
        }
        if matches!(request.account, Some(EditChange::Set(_))) {
            return Frame::NoAccount;
        }
        let new_name = match name {
            None => inner.tools[at].name.clone(),
            Some(EditChange::Delete) => None,
            Some(EditChange::Set(n)) => Some(n.clone()),
        };
        if new_name.is_some() && inner.tools.iter().enumerate().any(|(i, t)| i != at && t.name == new_name) {
            return Frame::InUse;
        }
        let made = &create.inner;
        let volume_mounts = changed(&made.provider.as_ref().map(|p| p.volume_mounts.clone()).unwrap_or_default(), volume_mounts);
        let fuse_file_mounts = changed(&made.fuse_file_mounts, fuse_file_mounts);
        let fuse_directory_mounts = changed(&made.fuse_directory_mounts, fuse_directory_mounts);
        let checked = {
            let fuse: Vec<&FuseMount> = fuse_file_mounts.iter().chain(&fuse_directory_mounts).collect();
            check_mounts(&inner, "", made.provider.as_ref().map(|p| &p.identity), &volume_mounts, &fuse)
        };
        if let Err(reason) = checked {
            return Frame::Error(wire_error(reason));
        }
        let tool = &mut inner.tools[at];
        tool.name = new_name;
        if let Some(create) = tool.create.as_mut() {
            if let Some(p) = create.inner.provider.as_mut() {
                p.volume_mounts = volume_mounts;
            }
            create.inner.fuse_file_mounts = fuse_file_mounts;
            create.inner.fuse_directory_mounts = fuse_directory_mounts;
        }
        inner.changed();
        Frame::Edited
    }

    async fn standalone_tools_register(&self, request: standalone_tools::register::client::request::Frame) -> standalone_tools::register::server::response::Frame {
        use standalone_tools::list::server::response::Origin;
        use standalone_tools::register::server::response::Frame;
        let mut inner = self.lock();
        if request.name.is_some() && inner.tools.iter().any(|t| t.name == request.name) {
            return Frame::InUse;
        }
        let joined = json!({ "daemon": request.daemon, "account": request.account, "tool": request.tool }).to_string();
        let index = inner.next_index(&joined);
        let origin = Origin::Connected { daemon: request.daemon, account: request.account, tool: request.tool, index };
        inner.tools.push(ToolState { name: request.name, origin, create: None, creator: client(), created: Utc::now(), agents: Vec::new(), templates: Vec::new(), last_active: None });
        Frame::Registered
    }

    async fn standalone_tools_attach_agent_instance(
        &self,
        request: standalone_tools::attach_agent_instance::client::request::Frame,
    ) -> standalone_tools::attach_agent_instance::server::response::Frame {
        use standalone_tools::attach_agent_instance::server::response::Frame;
        let mut inner = self.lock();
        let Some(agent) = inner.find(&request.agent) else { return Frame::NoAgent };
        let Some(at) = inner.find_tool(&request.tool) else { return Frame::NoTool };
        let tool = &mut inner.tools[at];
        if !tool.agents.contains(&agent) {
            tool.agents.push(agent);
        }
        inner.changed();
        Frame::Attached
    }

    async fn standalone_tools_detach_agent_instance(
        &self,
        request: standalone_tools::detach_agent_instance::client::request::Frame,
    ) -> standalone_tools::detach_agent_instance::server::response::Frame {
        use standalone_tools::detach_agent_instance::server::response::Frame;
        let mut inner = self.lock();
        let Some(agent) = inner.find(&request.agent) else { return Frame::NoAgent };
        let Some(at) = inner.find_tool(&request.tool) else { return Frame::NoTool };
        inner.tools[at].agents.retain(|a| a != &agent);
        inner.changed();
        Frame::Detached
    }

    async fn standalone_tools_attach_agent_template(
        &self,
        request: standalone_tools::attach_agent_template::client::request::Frame,
    ) -> standalone_tools::attach_agent_template::server::response::Frame {
        use standalone_tools::attach_agent_template::server::response::Frame;
        let mut inner = self.lock();
        if !inner.templates.contains_key(&request.template) {
            return Frame::NoTemplate;
        }
        let Some(at) = inner.find_tool(&request.tool) else { return Frame::NoTool };
        let tool = &mut inner.tools[at];
        if !tool.templates.contains(&request.template) {
            tool.templates.push(request.template);
        }
        inner.changed();
        Frame::Attached
    }

    async fn standalone_tools_detach_agent_template(
        &self,
        request: standalone_tools::detach_agent_template::client::request::Frame,
    ) -> standalone_tools::detach_agent_template::server::response::Frame {
        use standalone_tools::detach_agent_template::server::response::Frame;
        let mut inner = self.lock();
        if !inner.templates.contains_key(&request.template) {
            return Frame::NoTemplate;
        }
        let Some(at) = inner.find_tool(&request.tool) else { return Frame::NoTool };
        inner.tools[at].templates.retain(|t| t != &request.template);
        inner.changed();
        Frame::Detached
    }

    async fn standalone_tools_delete(&self, request: standalone_tools::delete::client::request::Frame) -> standalone_tools::delete::server::response::Frame {
        use standalone_tools::delete::server::response::Frame;
        let mut inner = self.lock();
        let Some(at) = inner.find_tool(&request.tool) else { return Frame::NotFound };
        if !inner.tools[at].agents.is_empty() || !inner.tools[at].templates.is_empty() {
            return Frame::Attached;
        }
        inner.tools.remove(at);
        Frame::Deleted
    }

    fn standalone_tools_list(
        &self,
        request: standalone_tools::list::client::request::Frame,
        cancel: CancellationToken,
    ) -> Frames<standalone_tools::list::server::response::Frame> {
        use standalone_tools::list::server::response::{Frame, Tool};
        let inner = self.lock();
        let mut frames: Vec<Frame> = inner
            .tools
            .iter()
            .filter(|t| request.filter.names.is_empty() || t.name.as_ref().is_some_and(|n| request.filter.names.contains(n)))
            .take(request.count.map_or(usize::MAX, |n| n as usize))
            .map(|t| {
                let attached_agent_instances = t
                    .agents
                    .iter()
                    .filter_map(|id| inner.agents.get(id))
                    .map(|a| key::Agent { template: a.create.inner.template.clone(), index: a.index, name: a.create.name.clone() })
                    .collect();
                Frame::Added(Tool {
                    name: t.name.clone(),
                    origin: t.origin.clone(),
                    creator: t.creator.clone(),
                    created: t.created,
                    active: inner.tool_active(t),
                    last_active: t.last_active,
                    attached_agent_instances,
                    attached_agent_templates: t.templates.clone(),
                    tags: Vec::new(),
                })
            })
            .collect();
        frames.push(Frame::Listed);
        Box::pin(stream::iter(frames).chain(open_until(cancel)))
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
    fn volumes_list(&self, on: &Identity, cancel: CancellationToken) -> Frames<volumes::list::server::response::Frame> {
        use volumes::list::server::response::Frame;
        let frames = match self.machine(on) {
            Ok(store) => store.list().into_iter().map(Frame::Added).chain([Frame::Listed]).collect(),
            Err(error) => vec![Frame::Error(error)],
        };
        Box::pin(stream::iter(frames).chain(open_until(cancel)))
    }

    async fn volumes_stat(&self, on: &Identity, request: volumes::stat::client::request::Frame) -> volumes::stat::server::response::Frame {
        use volumes::stat::server::response::Frame;
        let store = match self.machine(on) {
            Ok(store) => store,
            Err(error) => return Frame::Error(error),
        };
        match self.held(on, &request.name) {
            Some((_, Hold::Mounted)) => return Frame::Mounted,
            Some((_, Hold::Served)) => return Frame::FuseServed,
            None => {}
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
                Some((_, Hold::Mounted)) => vec![ReadFrame::Mounted],
                Some((_, Hold::Served)) => vec![ReadFrame::FuseServed],
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
        match self.held(on, &request.name) {
            Some((_, Hold::Mounted)) => return Frame::Mounted,
            Some((_, Hold::Served)) => return Frame::FuseServed,
            None => {}
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
        match self.held(on, &request.name) {
            Some((_, Hold::Mounted)) => return Frame::Mounted,
            Some((_, Hold::Served)) => return Frame::FuseServed,
            None => {}
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
        match self.held(on, &request.name) {
            Some((_, Hold::Mounted)) => return Frame::Mounted,
            Some((_, Hold::Served)) => return Frame::FuseServed,
            None => {}
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
        match self.held(on, &request.name) {
            Some((_, Hold::Mounted)) => return Frame::Mounted,
            Some((_, Hold::Served)) => return Frame::FuseServed,
            None => {}
        }
        match store.delete(&request.name) {
            Ok(()) => Frame::Deleted,
            Err(reason) => Frame::Error(wire_error(reason)),
        }
    }
}
