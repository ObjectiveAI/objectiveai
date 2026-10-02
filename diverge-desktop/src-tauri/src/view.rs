//! What the screen is handed: this app's own types, not the daemon's.
//!
//! Every type here writes its own TypeScript (`cargo test` exports them to
//! `src/bindings/`). Every conversion from a daemon type is an exhaustive
//! `match` with no catch-all, so a variant the SDK adds fails this build
//! instead of drifting past the screen.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use ts_rs::TS;

use diverge_sdk::daemon::endpoints::agents;
use diverge_sdk::daemon::endpoints::agents::logs::server::response::{Identity, Item, ItemWrapper};
use diverge_sdk::provider::endpoints::containers::agents::run::server::response::AgenticLoopChunk;
use diverge_sdk::shared::error::Error as WireError;
use diverge_sdk::shared::filetree::response::Node;
use diverge_sdk::provider::endpoints::volumes;
use diverge_sdk::provider::endpoints::volumes::Mode;

const OUT: &str = "../../src/bindings/";

// --- shared ------------------------------------------------------------

/// A provider, only as the daemon names it.
#[derive(Serialize, Deserialize, TS, Clone, Debug, PartialEq)]
#[serde(tag = "kind", rename_all = "snake_case")]
#[ts(export, export_to = "../../src/bindings/")]
pub enum ProviderView {
    Outgoing { address: String },
    IncomingUnbrokered { identity: String },
}

impl From<&Identity> for ProviderView {
    fn from(identity: &Identity) -> Self {
        match identity {
            Identity::Outgoing { address } => ProviderView::Outgoing { address: address.clone() },
            Identity::IncomingUnbrokered { identity } => ProviderView::IncomingUnbrokered { identity: identity.clone() },
        }
    }
}

impl From<&ProviderView> for Identity {
    fn from(view: &ProviderView) -> Self {
        match view {
            ProviderView::Outgoing { address } => Identity::Outgoing { address: address.clone() },
            ProviderView::IncomingUnbrokered { identity } => Identity::IncomingUnbrokered { identity: identity.clone() },
        }
    }
}

/// What went wrong, in words. The daemon's error is any JSON; its
/// `message` if it has one, the whole thing otherwise.
pub fn error_text(error: &WireError) -> String {
    match &error.0 {
        Value::String(s) => s.clone(),
        Value::Object(o) => o.get("message").and_then(Value::as_str).map(str::to_owned).unwrap_or_else(|| error.0.to_string()),
        other => other.to_string(),
    }
}

// --- the catalog ---------------------------------------------------------

#[derive(Serialize, TS, Clone, Debug)]
#[ts(export, export_to = "../../src/bindings/")]
pub struct ImageKindView {
    pub key: String,
    pub image_name: String,
    pub digest: String,
    /// JSON Schema of the image's settings, from the SDK's source.
    #[ts(type = "Record<string, unknown>")]
    pub schema: Value,
}

// --- agents ------------------------------------------------------------

#[derive(Serialize, TS, Clone, Debug)]
#[ts(export, export_to = "../../src/bindings/")]
pub struct AgentView {
    pub name: String,
    pub image_name: String,
    pub digest: String,
    pub created: String,
    pub active: bool,
    pub last_active: Option<String>,
    pub provider: Option<ProviderView>,
    #[ts(type = "number")]
    pub logs_index: u64,
    /// The daemon's tools attached to it.
    pub tools: Vec<String>,
}

/// What this app last stated an agent mounts, in the daemon's own shapes.
/// Ours: the daemon's listing does not repeat a create's mounts ("the
/// create's, and is not repeated here"), and an edit states them all anew,
/// so the app keeps what it said. An agent made elsewhere has no record,
/// and the app does not offer to change its mounts.
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct AgentMounts {
    pub pinned: Option<Identity>,
    pub volume_mounts: Vec<agents::create::client::request::VolumeMount>,
    pub fuse_file_mounts: Vec<agents::create::client::request::FuseMount>,
    pub fuse_directory_mounts: Vec<agents::create::client::request::FuseMount>,
}

impl AgentMounts {
    pub fn of_create(request: &agents::create::client::request::Frame) -> Self {
        AgentMounts {
            pinned: request.provider.as_ref().map(|p| p.identity.clone()),
            volume_mounts: request.provider.as_ref().map(|p| p.volume_mounts.clone()).unwrap_or_default(),
            fuse_file_mounts: request.fuse_file_mounts.clone(),
            fuse_directory_mounts: request.fuse_directory_mounts.clone(),
        }
    }

    /// The same machine; the three lists as the edit stated them.
    pub fn edited(&self, request: &agents::edit::client::request::Frame) -> Self {
        AgentMounts {
            pinned: self.pinned.clone(),
            volume_mounts: request.volume_mounts.clone(),
            fuse_file_mounts: request.fuse_file_mounts.clone(),
            fuse_directory_mounts: request.fuse_directory_mounts.clone(),
        }
    }

    pub fn view(&self) -> MountsView {
        let own = |m: &agents::create::client::request::VolumeMount| MountView {
            provider: None,
            volume_name: m.volume_name.clone(),
            volume_relative_path: slashed(&m.volume_relative_path),
            volume_mode: m.volume_mode.into(),
            overlay_disk: None,
            container_path: slashed(&m.container_path),
        };
        let live = |m: &agents::create::client::request::FuseMount| MountView {
            provider: Some((&m.provider).into()),
            volume_name: m.volume_name.clone(),
            volume_relative_path: slashed(&m.volume_relative_path),
            volume_mode: m.volume_mode.into(),
            overlay_disk: m.overlay_disk,
            container_path: slashed(&m.container_path),
        };
        MountsView {
            pinned: self.pinned.as_ref().map(Into::into),
            volume_mounts: self.volume_mounts.iter().map(own).collect(),
            fuse_file_mounts: self.fuse_file_mounts.iter().map(live).collect(),
            fuse_directory_mounts: self.fuse_directory_mounts.iter().map(live).collect(),
        }
    }
}

#[derive(Serialize, TS, Clone, Debug)]
#[ts(export, export_to = "../../src/bindings/")]
pub struct MountsView {
    /// The machine it is pinned to, for life; none, it runs wherever the
    /// daemon chooses and mounts no machine's volumes directly.
    pub pinned: Option<ProviderView>,
    pub volume_mounts: Vec<MountView>,
    pub fuse_file_mounts: Vec<MountView>,
    pub fuse_directory_mounts: Vec<MountView>,
}

/// One volume mounted in an agent: its pinned machine's (no `provider`),
/// or any machine's, served live across the daemon.
#[derive(Serialize, TS, Clone, Debug)]
#[ts(export, export_to = "../../src/bindings/")]
pub struct MountView {
    pub provider: Option<ProviderView>,
    pub volume_name: String,
    pub volume_relative_path: String,
    pub volume_mode: VolumeMode,
    #[ts(type = "number | null")]
    pub overlay_disk: Option<u64>,
    pub container_path: String,
}

/// What becomes of a change to a volume: the provider's three modes.
#[derive(Serialize, Deserialize, TS, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
#[ts(export, export_to = "../../src/bindings/")]
pub enum VolumeMode {
    /// Every change is kept. One user at a time.
    Persistent,
    /// Every run starts from the volume as it is; its changes are dropped.
    Ephemeral,
    /// Agents read it; nothing they do changes it.
    ReadOnly,
}

impl From<Mode> for VolumeMode {
    fn from(mode: Mode) -> Self {
        match mode {
            Mode::Persistent => VolumeMode::Persistent,
            Mode::Ephemeral => VolumeMode::Ephemeral,
            Mode::ReadOnly => VolumeMode::ReadOnly,
        }
    }
}

impl From<VolumeMode> for Mode {
    fn from(mode: VolumeMode) -> Self {
        match mode {
            VolumeMode::Persistent => Mode::Persistent,
            VolumeMode::Ephemeral => Mode::Ephemeral,
            VolumeMode::ReadOnly => Mode::ReadOnly,
        }
    }
}

fn slashed(parts: &[String]) -> String {
    parts.join("/")
}

#[derive(Serialize, TS, Clone, Debug, Default)]
#[ts(export, export_to = "../../src/bindings/")]
pub struct AgentsListed {
    pub agents: Vec<AgentView>,
    pub errors: Vec<String>,
}

pub fn listed(frames: Vec<agents::list::server::response::Frame>) -> AgentsListed {
    use agents::list::server::response::Frame;
    let mut out = AgentsListed::default();
    for frame in frames {
        match frame {
            Frame::Agent(agent) => out.agents.push(AgentView {
                name: agent.name,
                image_name: agent.image.name,
                digest: agent.image.digest,
                created: agent.created.to_rfc3339(),
                active: agent.active,
                last_active: agent.last_active.map(|t| t.to_rfc3339()),
                provider: agent.provider.as_ref().map(|p| (&p.identity).into()),
                logs_index: agent.logs_index,
                tools: agent.tools,
            }),
            Frame::Error(error) => out.errors.push(error_text(&error)),
        }
    }
    out
}

/// What the screen sends to make an agent.
#[derive(Deserialize, TS, Clone, Debug)]
#[ts(export, export_to = "../../src/bindings/")]
pub struct CreateAgentInput {
    pub name: String,
    pub image_kind: String,
    #[ts(type = "number")]
    pub memory: u64,
    #[ts(type = "number")]
    pub disk: u64,
    pub provider: Option<ProviderView>,
    pub volume_mounts: Vec<VolumeMountInput>,
    pub fuse_file_mounts: Vec<FuseMountInput>,
    pub fuse_directory_mounts: Vec<FuseMountInput>,
    #[ts(type = "unknown")]
    pub arguments: Value,
}

#[derive(Deserialize, TS, Clone, Debug)]
#[ts(export, export_to = "../../src/bindings/")]
pub struct VolumeMountInput {
    pub volume_name: String,
    pub volume_relative_path: String,
    pub volume_mode: VolumeMode,
    pub container_path: String,
}

/// A live share: any machine's volume (and a folder or a file in it),
/// served across the daemon, as the agent sees it.
#[derive(Deserialize, TS, Clone, Debug)]
#[ts(export, export_to = "../../src/bindings/")]
pub struct FuseMountInput {
    pub provider: ProviderView,
    pub volume_name: String,
    pub volume_relative_path: String,
    pub volume_mode: VolumeMode,
    /// For a volume that starts fresh each run: how many bytes its changes may take.
    #[ts(type = "number | null")]
    pub overlay_disk: Option<u64>,
    pub container_path: String,
}

impl FuseMountInput {
    fn into_wire(self) -> agents::create::client::request::FuseMount {
        agents::create::client::request::FuseMount {
            provider: (&self.provider).into(),
            volume_name: self.volume_name,
            volume_relative_path: components(&self.volume_relative_path),
            volume_mode: self.volume_mode.into(),
            overlay_disk: self.overlay_disk,
            container_path: components(&self.container_path),
        }
    }
}

impl VolumeMountInput {
    fn into_wire(self) -> agents::create::client::request::VolumeMount {
        agents::create::client::request::VolumeMount {
            volume_name: self.volume_name,
            volume_relative_path: components(&self.volume_relative_path),
            volume_mode: self.volume_mode.into(),
            container_path: components(&self.container_path),
        }
    }
}

fn components(path: &str) -> Vec<String> {
    path.split('/').filter(|p| !p.is_empty()).map(str::to_owned).collect()
}

impl CreateAgentInput {
    /// Into the daemon's own create. Every field named, so a field the SDK
    /// adds fails this build.
    pub fn into_request(self) -> Result<agents::create::client::request::Frame, String> {
        use agents::create::client::request::{Frame, Image, Provider};
        let kind = crate::catalog::ALL
            .into_iter()
            .find(|k| k.key() == self.image_kind)
            .ok_or_else(|| format!("no image called {}", self.image_kind))?;
        let fuse = |mounts: Vec<FuseMountInput>| mounts.into_iter().map(FuseMountInput::into_wire).collect::<Vec<_>>();
        Ok(Frame {
            image: Image { name: kind.image_name(), digest: crate::catalog::UNBUILT_DIGEST.into() },
            memory: self.memory,
            disk: self.disk,
            provider: self.provider.as_ref().map(|p| Provider {
                identity: p.into(),
                volume_mounts: self.volume_mounts.into_iter().map(VolumeMountInput::into_wire).collect(),
            }),
            fuse_file_mounts: fuse(self.fuse_file_mounts),
            fuse_directory_mounts: fuse(self.fuse_directory_mounts),
            arguments: self.arguments,
            name: self.name,
        })
    }
}

#[derive(Serialize, TS, Clone, Debug)]
#[serde(tag = "outcome", rename_all = "snake_case")]
#[ts(export, export_to = "../../src/bindings/")]
pub enum CreateOutcome {
    Created,
    InUse,
    Error { message: String },
}

/// What the screen sends to change an agent's mounts: all three lists,
/// stated anew.
#[derive(Deserialize, TS, Clone, Debug)]
#[ts(export, export_to = "../../src/bindings/")]
pub struct EditMountsInput {
    pub name: String,
    pub volume_mounts: Vec<VolumeMountInput>,
    pub fuse_file_mounts: Vec<FuseMountInput>,
    pub fuse_directory_mounts: Vec<FuseMountInput>,
}

impl EditMountsInput {
    pub fn into_request(self) -> agents::edit::client::request::Frame {
        agents::edit::client::request::Frame {
            name: self.name,
            volume_mounts: self.volume_mounts.into_iter().map(VolumeMountInput::into_wire).collect(),
            fuse_file_mounts: self.fuse_file_mounts.into_iter().map(FuseMountInput::into_wire).collect(),
            fuse_directory_mounts: self.fuse_directory_mounts.into_iter().map(FuseMountInput::into_wire).collect(),
        }
    }
}

#[derive(Serialize, TS, Clone, Debug)]
#[serde(tag = "outcome", rename_all = "snake_case")]
#[ts(export, export_to = "../../src/bindings/")]
pub enum EditOutcome {
    Edited,
    NotFound,
    /// It is working; ask again once it stops.
    Active,
    Error { message: String },
}

impl From<agents::edit::server::response::Frame> for EditOutcome {
    fn from(frame: agents::edit::server::response::Frame) -> Self {
        use agents::edit::server::response::Frame;
        match frame {
            Frame::Edited => EditOutcome::Edited,
            Frame::NotFound => EditOutcome::NotFound,
            Frame::Active => EditOutcome::Active,
            Frame::Error(error) => EditOutcome::Error { message: error_text(&error) },
        }
    }
}

impl From<agents::create::server::response::Frame> for CreateOutcome {
    fn from(frame: agents::create::server::response::Frame) -> Self {
        use agents::create::server::response::Frame;
        match frame {
            Frame::Created => CreateOutcome::Created,
            Frame::InUse => CreateOutcome::InUse,
            Frame::Error(error) => CreateOutcome::Error { message: error_text(&error) },
        }
    }
}

#[derive(Serialize, TS, Clone, Debug)]
#[serde(tag = "outcome", rename_all = "snake_case")]
#[ts(export, export_to = "../../src/bindings/")]
pub enum DeleteOutcome {
    Deleted,
    NotFound,
    Active,
    Error { message: String },
}

impl From<agents::delete::server::response::Frame> for DeleteOutcome {
    fn from(frame: agents::delete::server::response::Frame) -> Self {
        use agents::delete::server::response::Frame;
        match frame {
            Frame::Deleted => DeleteOutcome::Deleted,
            Frame::NotFound => DeleteOutcome::NotFound,
            Frame::Active => DeleteOutcome::Active,
            Frame::Error(error) => DeleteOutcome::Error { message: error_text(&error) },
        }
    }
}

#[derive(Serialize, TS, Clone, Debug)]
#[serde(tag = "outcome", rename_all = "snake_case")]
#[ts(export, export_to = "../../src/bindings/")]
pub enum MessageOutcome {
    Delivered,
    Cancelled,
    Error { message: String },
}

impl From<agents::message::server::response::Frame> for MessageOutcome {
    fn from(frame: agents::message::server::response::Frame) -> Self {
        use agents::message::server::response::Frame;
        match frame {
            Frame::Delivered => MessageOutcome::Delivered,
            Frame::Cancelled => MessageOutcome::Cancelled,
            Frame::Error(error) => MessageOutcome::Error { message: error_text(&error) },
        }
    }
}

// --- the log -------------------------------------------------------------

/// One kept item, flattened for the screen.
#[derive(Serialize, TS, Clone, Debug)]
#[ts(export, export_to = "../../src/bindings/")]
pub struct LogEntry {
    #[ts(type = "number")]
    pub logs_index: u64,
    pub created: String,
    pub item: LogItem,
}

#[derive(Serialize, TS, Clone, Debug)]
#[serde(tag = "kind", rename_all = "snake_case")]
#[ts(export, export_to = "../../src/bindings/")]
pub enum LogItem {
    UserText { key: String, text: String },
    UserImage { key: String, mime_type: String },
    UserAudio { key: String, mime_type: String },
    UserResource { key: String, uri: String },
    UserResourceLink { key: String, uri: String, name: String },
    Reasoning { parent: Option<String>, text: String },
    Text { parent: Option<String>, text: String },
    Image { parent: Option<String>, mime_type: String, data: String },
    Audio { parent: Option<String>, mime_type: String },
    ToolCall { parent: Option<String>, id: String, name: String, arguments: Option<String> },
    ToolResponse { parent: Option<String>, id: String, is_error: bool, text: String },
    Refusal { parent: Option<String>, text: String },
    Usage {
        #[ts(type = "number")]
        prompt_tokens: u64,
        #[ts(type = "number")]
        completion_tokens: u64,
        #[ts(type = "number")]
        total_tokens: u64,
    },
    Notification {
        fatal: bool,
        #[ts(type = "unknown")]
        message: Value,
    },
    Error { message: String },
    Active { provider: ProviderView },
    Inactive { provider: ProviderView },
}

fn field(value: &Value, name: &str) -> String {
    value.get(name).and_then(Value::as_str).unwrap_or_default().to_owned()
}

fn json<T: Serialize>(inner: &T) -> Value {
    serde_json::to_value(inner).unwrap_or(Value::Null)
}

/// The text parts of a tool's answer, joined.
fn content_text(result: &Value) -> String {
    result
        .get("content")
        .and_then(Value::as_array)
        .map(|parts| {
            parts
                .iter()
                .map(|part| match part.get("type").and_then(Value::as_str) {
                    Some("text") => field(part, "text"),
                    Some(other) => format!("[{other}]"),
                    None => String::new(),
                })
                .collect::<Vec<_>>()
                .join("\n")
        })
        .unwrap_or_default()
}

impl From<&ItemWrapper> for LogEntry {
    fn from(wrapper: &ItemWrapper) -> Self {
        let item = match &wrapper.item {
            Item::Chunk(chunk) => match chunk {
                AgenticLoopChunk::AssistantReasoning(c) => LogItem::Reasoning { parent: c.parent_tool_call_id.clone(), text: field(&json(&c.inner), "text") },
                AgenticLoopChunk::AssistantTextContent(c) => LogItem::Text { parent: c.parent_tool_call_id.clone(), text: field(&json(&c.inner), "text") },
                AgenticLoopChunk::AssistantImageContent(c) => {
                    let v = json(&c.inner);
                    LogItem::Image { parent: c.parent_tool_call_id.clone(), mime_type: field(&v, "mimeType"), data: field(&v, "data") }
                }
                AgenticLoopChunk::AssistantAudioContent(c) => LogItem::Audio { parent: c.parent_tool_call_id.clone(), mime_type: field(&json(&c.inner), "mimeType") },
                AgenticLoopChunk::AssistantToolCall(c) => LogItem::ToolCall { parent: c.parent_tool_call_id.clone(), id: c.id.clone(), name: c.name.clone(), arguments: c.arguments.clone() },
                AgenticLoopChunk::AssistantRefusal(c) => LogItem::Refusal { parent: c.parent_tool_call_id.clone(), text: field(&json(&c.inner), "text") },
                AgenticLoopChunk::ToolResponse(c) => {
                    let v = json(&c.inner);
                    LogItem::ToolResponse {
                        parent: c.parent_tool_call_id.clone(),
                        id: c.id.clone(),
                        is_error: v.get("isError").and_then(Value::as_bool).unwrap_or(false),
                        text: content_text(&v),
                    }
                }
                AgenticLoopChunk::UserTextContent(c) => LogItem::UserText { key: c.key.clone(), text: field(&json(&c.inner), "text") },
                AgenticLoopChunk::UserImageContent(c) => LogItem::UserImage { key: c.key.clone(), mime_type: field(&json(&c.inner), "mimeType") },
                AgenticLoopChunk::UserAudioContent(c) => LogItem::UserAudio { key: c.key.clone(), mime_type: field(&json(&c.inner), "mimeType") },
                AgenticLoopChunk::UserResource(c) => {
                    let v = json(&c.inner);
                    let uri = v.get("resource").map(|r| field(r, "uri")).unwrap_or_default();
                    LogItem::UserResource { key: c.key.clone(), uri }
                }
                AgenticLoopChunk::UserResourceLink(c) => {
                    let v = json(&c.inner);
                    LogItem::UserResourceLink { key: c.key.clone(), uri: field(&v, "uri"), name: field(&v, "name") }
                }
                AgenticLoopChunk::Usage(c) => LogItem::Usage { prompt_tokens: c.prompt_tokens, completion_tokens: c.completion_tokens, total_tokens: c.total_tokens },
                AgenticLoopChunk::Notification(c) => LogItem::Notification { fatal: c.is_fatal, message: c.message.clone() },
            },
            Item::Error(error) => LogItem::Error { message: error_text(&error.error) },
            Item::Active(active) => LogItem::Active { provider: (&active.provider.identity).into() },
            Item::Inactive(inactive) => LogItem::Inactive { provider: (&inactive.provider.identity).into() },
        };
        LogEntry { logs_index: wrapper.logs_index, created: wrapper.created.to_rfc3339(), item }
    }
}

/// One value a logs scope sent: an item as it is, or — when a `jq`
/// program reshaped it — whatever the program yielded.
#[derive(Serialize, TS, Clone, Debug)]
#[serde(tag = "event", rename_all = "snake_case")]
#[ts(export, export_to = "../../src/bindings/")]
pub enum LogEvent {
    Entry { entry: LogEntry },
    Value {
        #[ts(type = "unknown")]
        value: Value,
    },
    Error { message: String },
    End,
}

impl From<agents::logs::server::response::Frame> for LogEvent {
    fn from(frame: agents::logs::server::response::Frame) -> Self {
        use agents::logs::server::response::Frame;
        match frame {
            Frame::Value(value) => match serde_json::from_value::<ItemWrapper>(value.clone()) {
                Ok(wrapper) => LogEvent::Entry { entry: (&wrapper).into() },
                Err(_) => LogEvent::Value { value },
            },
            Frame::Error(error) => LogEvent::Error { message: error_text(&error) },
        }
    }
}

/// What the screen asks the log for. Mirrors the daemon's logs request.
#[derive(Deserialize, Serialize, TS, Clone, Debug, Default)]
#[ts(export, export_to = "../../src/bindings/")]
pub struct LogsQuery {
    pub name: String,
    #[ts(type = "number | null")]
    pub logs_index_from: Option<u64>,
    #[ts(type = "number | null")]
    pub logs_index_to: Option<u64>,
    pub created_from: Option<String>,
    pub created_to: Option<String>,
    /// One kind of item, by its wire name (e.g. "tool_response").
    pub item_type: Option<String>,
    pub jq: Option<String>,
    #[ts(type = "number | null")]
    pub count: Option<u64>,
    pub watch: bool,
}

impl LogsQuery {
    pub fn into_request(self) -> Result<agents::logs::client::request::Frame, String> {
        let time = |s: Option<String>| -> Result<_, String> {
            s.filter(|s| !s.trim().is_empty())
                .map(|s| chrono::DateTime::parse_from_rfc3339(&s).map(|t| t.to_utc()).map_err(|e| format!("\"{s}\" is not a time: {e}")))
                .transpose()
        };
        let item_type = self
            .item_type
            .filter(|t| !t.is_empty())
            .map(|t| serde_json::from_value(Value::String(t.clone())).map_err(|_| format!("no kind of item called \"{t}\"")))
            .transpose()?;
        Ok(agents::logs::client::request::Frame {
            name: self.name,
            logs_index_from: self.logs_index_from,
            logs_index_to: self.logs_index_to,
            created_from: time(self.created_from)?,
            created_to: time(self.created_to)?,
            r#type: item_type,
            jq: self.jq.filter(|j| !j.trim().is_empty()),
            count: self.count,
            watch: Some(self.watch),
        })
    }
}

// --- storage (the daemon's volumes) --------------------------------------

#[derive(Serialize, TS, Clone, Debug)]
#[serde(tag = "kind", rename_all = "snake_case")]
#[ts(export, export_to = "../../src/bindings/")]
pub enum FileNode {
    File {
        name: String,
        #[ts(type = "number | null")]
        size: Option<u64>,
        #[ts(type = "number | null")]
        modified_at: Option<u64>,
    },
    Directory {
        name: String,
        children: Vec<FileNode>,
    },
    Symlink {
        name: String,
        target: Vec<String>,
    },
}

impl From<&Node> for FileNode {
    fn from(node: &Node) -> Self {
        match node {
            Node::File { name, size, created_at: _, modified_at } => FileNode::File { name: name.clone(), size: *size, modified_at: *modified_at },
            Node::Directory { name, created_at: _, modified_at: _, changes: _, children } => {
                FileNode::Directory { name: name.clone(), children: children.iter().map(Into::into).collect() }
            }
            Node::Symlink { name, path, created_at: _, modified_at: _ } => FileNode::Symlink { name: name.clone(), target: path.clone() },
        }
    }
}

#[derive(Serialize, TS, Clone, Debug)]
#[ts(export, export_to = "../../src/bindings/")]
pub struct VolumeView {
    pub name: String,
    /// What it reserves, in bytes.
    #[ts(type = "number")]
    pub bytes: u64,
    pub created: String,
    /// What becomes of what an agent writes into it.
    pub mode: VolumeMode,
}

impl From<&diverge_sdk::provider::endpoints::volumes::list::server::response::Volume> for VolumeView {
    fn from(v: &diverge_sdk::provider::endpoints::volumes::list::server::response::Volume) -> Self {
        let created = chrono::DateTime::from_timestamp(v.created as i64, 0).map(|t| t.to_rfc3339()).unwrap_or_default();
        VolumeView { name: v.name.clone(), bytes: v.bytes, created, mode: v.mode.into() }
    }
}

#[derive(Serialize, TS, Clone, Debug)]
#[serde(tag = "outcome", rename_all = "snake_case")]
#[ts(export, export_to = "../../src/bindings/")]
pub enum VolumesListed {
    Volumes { volumes: Vec<VolumeView> },
    Error { message: String },
}

impl From<volumes::list::server::response::Frame> for VolumesListed {
    fn from(frame: volumes::list::server::response::Frame) -> Self {
        use volumes::list::server::response::Frame;
        match frame {
            Frame::Volumes(list) => VolumesListed::Volumes { volumes: list.iter().map(Into::into).collect() },
            Frame::Error(error) => VolumesListed::Error { message: error_text(&error) },
        }
    }
}

#[derive(Serialize, TS, Clone, Debug)]
#[serde(tag = "outcome", rename_all = "snake_case")]
#[ts(export, export_to = "../../src/bindings/")]
pub enum VolumeStat {
    Stat {
        volume: VolumeView,
        #[ts(type = "number")]
        bytes_used: u64,
    },
    Error { message: String },
}

impl From<volumes::stat::server::response::Frame> for VolumeStat {
    fn from(frame: volumes::stat::server::response::Frame) -> Self {
        use volumes::stat::server::response::Frame;
        match frame {
            Frame::Stat(stat) => VolumeStat::Stat { volume: (&stat.volume).into(), bytes_used: stat.bytes_used },
            Frame::Error(error) => VolumeStat::Error { message: error_text(&error) },
        }
    }
}

#[derive(Serialize, TS, Clone, Debug)]
#[serde(tag = "outcome", rename_all = "snake_case")]
#[ts(export, export_to = "../../src/bindings/")]
pub enum VolumeTree {
    Tree { nodes: Vec<FileNode> },
    Error { message: String },
}

impl From<volumes::filetree::server::response::Frame> for VolumeTree {
    fn from(frame: volumes::filetree::server::response::Frame) -> Self {
        use volumes::filetree::server::response::Frame;
        match frame {
            Frame::Tree(nodes) => VolumeTree::Tree { nodes: nodes.iter().map(Into::into).collect() },
            Frame::Error(error) => VolumeTree::Error { message: error_text(&error) },
        }
    }
}

/// How much room there is: to make a volume, or for one to grow into.
#[derive(Serialize, TS, Clone, Debug)]
#[serde(tag = "outcome", rename_all = "snake_case")]
#[ts(export, export_to = "../../src/bindings/")]
pub enum Capacity {
    Capacity {
        #[ts(type = "number")]
        bytes: u64,
    },
    Error { message: String },
}

impl From<volumes::create_capacity::server::response::Frame> for Capacity {
    fn from(frame: volumes::create_capacity::server::response::Frame) -> Self {
        use volumes::create_capacity::server::response::Frame;
        match frame {
            Frame::Capacity(bytes) => Capacity::Capacity { bytes },
            Frame::Error(error) => Capacity::Error { message: error_text(&error) },
        }
    }
}

impl From<volumes::edit_capacity::server::response::Frame> for Capacity {
    fn from(frame: volumes::edit_capacity::server::response::Frame) -> Self {
        use volumes::edit_capacity::server::response::Frame;
        match frame {
            Frame::Capacity(bytes) => Capacity::Capacity { bytes },
            Frame::Error(error) => Capacity::Error { message: error_text(&error) },
        }
    }
}

/// Every volume change's answer, in the screen's words.
#[derive(Serialize, TS, Clone, Debug)]
#[serde(tag = "outcome", rename_all = "snake_case")]
#[ts(export, export_to = "../../src/bindings/")]
pub enum VolumeChange {
    Done,
    NotEnoughRoom,
    MoreContentThanThat,
    InUse,
    Error { message: String },
}

impl From<volumes::create::server::response::Frame> for VolumeChange {
    fn from(frame: volumes::create::server::response::Frame) -> Self {
        use volumes::create::server::response::Frame;
        match frame {
            Frame::Created => VolumeChange::Done,
            Frame::InsufficientCapacity => VolumeChange::NotEnoughRoom,
            Frame::Error(error) => VolumeChange::Error { message: error_text(&error) },
        }
    }
}

impl From<volumes::edit::server::response::Frame> for VolumeChange {
    fn from(frame: volumes::edit::server::response::Frame) -> Self {
        use volumes::edit::server::response::Frame;
        match frame {
            Frame::Edited => VolumeChange::Done,
            Frame::InsufficientCapacity => VolumeChange::NotEnoughRoom,
            Frame::ContentTooLarge => VolumeChange::MoreContentThanThat,
            Frame::Error(error) => VolumeChange::Error { message: error_text(&error) },
        }
    }
}

impl From<volumes::delete::server::response::Frame> for VolumeChange {
    fn from(frame: volumes::delete::server::response::Frame) -> Self {
        use volumes::delete::server::response::Frame;
        match frame {
            Frame::Deleted => VolumeChange::Done,
            Frame::Mounted => VolumeChange::InUse,
            Frame::Error(error) => VolumeChange::Error { message: error_text(&error) },
        }
    }
}

#[derive(Serialize, TS, Clone, Debug)]
#[serde(tag = "outcome", rename_all = "snake_case")]
#[ts(export, export_to = "../../src/bindings/")]
pub enum FileRead {
    Text {
        text: String,
        #[ts(type = "number")]
        bytes: u64,
    },
    Binary {
        #[ts(type = "number")]
        bytes: u64,
    },
    Error { message: String },
}

#[derive(Serialize, TS, Clone, Debug)]
#[serde(tag = "outcome", rename_all = "snake_case")]
#[ts(export, export_to = "../../src/bindings/")]
pub enum FileWritten {
    Written,
    Error { message: String },
}

impl From<volumes::write::server::response::Frame> for FileWritten {
    fn from(frame: volumes::write::server::response::Frame) -> Self {
        use volumes::write::server::response::Frame;
        match frame {
            Frame::Written(_) => FileWritten::Written,
            Frame::Error(error) => FileWritten::Error { message: error_text(&error) },
        }
    }
}

// --- Spaces (the social layer, on the provider protocol's tool rooms) ------

#[derive(Serialize, TS, Clone, Debug)]
#[ts(export, export_to = "../../src/bindings/")]
pub struct SpaceSummary {
    pub id: String,
    pub title: String,
    pub kind: String,
    pub host: ProviderView,
    pub host_name: String,
    pub mine: bool,
    pub online: bool,
    /// The name you go by there, and the key you seal with.
    pub you_are: String,
    pub you_key: String,
    /// The account that name is: a room under rules 2 knows you by it, on
    /// its member list and in what it records.
    pub you_account: Option<String>,
    /// Whether you're a fresh persona there, not your usual self.
    pub fresh: bool,
}

pub fn summary(e: &crate::spaces::SpaceEntry, identity: &crate::identity::Identity) -> SpaceSummary {
    let you = identity.who_in(&e.id.id).ok();
    SpaceSummary {
        id: e.id.id.clone(),
        title: e.title.clone(),
        kind: e.kind.clone(),
        host: (&e.host).into(),
        host_name: e.host_name.clone(),
        mine: e.mine,
        online: e.online,
        you_are: you.as_ref().map(|p| p.name.clone()).unwrap_or_default(),
        you_key: you.as_ref().map(|p| p.key.clone()).unwrap_or_default(),
        you_account: you.as_ref().and_then(|p| p.account.clone()),
        fresh: you.is_some_and(|p| !p.usual),
    }
}

/// Someone on a room's list: only those who chose to be.
#[derive(Serialize, Deserialize, TS, Clone, Debug)]
#[ts(export, export_to = "../../src/bindings/")]
pub struct MemberView {
    pub name: String,
    #[serde(default)]
    pub key: String,
    pub is_agent: bool,
    #[serde(default)]
    pub agent_of: Option<String>,
    /// For an agent: its person's key. Agents are told apart by key, never by name.
    #[serde(default)]
    pub agent_of_key: Option<String>,
    pub joined: String,
    #[serde(default)]
    pub last_acted: Option<String>,
    /// A mark of their key in this room, when someone else here goes by the
    /// same name (see [`crate::marks`]). The room doesn't say it; the app does.
    #[serde(default)]
    pub mark: Option<String>,
    /// Under rules 2, the keys on a person's current device list: any of
    /// them acts for them here. Empty under rules 1.
    #[serde(default)]
    pub devices: Vec<String>,
    /// For an agent of yours: its slot, which its allowances go by. The
    /// room doesn't say it; the app does.
    #[serde(default)]
    pub slot: Option<String>,
}

/// A room's verb: an MCP tool, rendered as a button with a generated form.
#[derive(Serialize, TS, Clone, Debug)]
#[ts(export, export_to = "../../src/bindings/")]
pub struct ToolView {
    pub name: String,
    pub title: String,
    pub description: String,
    #[ts(type = "Record<string, unknown>")]
    pub schema: Value,
    /// Only the room's host may use it; the room enforces that.
    pub host_only: bool,
}

impl From<&rmcp::model::Tool> for ToolView {
    fn from(t: &rmcp::model::Tool) -> Self {
        ToolView {
            name: t.name.to_string(),
            title: t.title.clone().unwrap_or_else(|| t.name.replace('_', " ")),
            description: t.description.as_deref().unwrap_or_default().to_owned(),
            schema: Value::Object((*t.input_schema).clone()),
            host_only: t.meta.as_ref().and_then(|m| m.0.get(diverge_desktop_room::room::META_HOST_ONLY)).and_then(Value::as_bool).unwrap_or(false),
        }
    }
}

#[derive(Serialize, TS, Clone, Debug)]
#[ts(export, export_to = "../../src/bindings/")]
pub struct SpaceView {
    pub summary: SpaceSummary,
    pub charter: String,
    pub members: Vec<MemberView>,
    pub tools: Vec<ToolView>,
    /// For a room that continues another: who was listed there and isn't
    /// here yet. Nobody is invited unless you send it.
    pub before: Vec<BeforeView>,
}

/// Someone from the room this one continues.
#[derive(Serialize, TS, Clone, Debug)]
#[ts(export, export_to = "../../src/bindings/")]
pub struct BeforeView {
    pub name: String,
    pub key: String,
    /// A direct room you share with them, to send the invite through.
    pub dm: Option<String>,
}

/// One object in a room's feed.
#[derive(Serialize, Deserialize, TS, Clone, Debug)]
#[ts(export, export_to = "../../src/bindings/")]
pub struct MoveView {
    pub id: String,
    pub kind: String,
    pub author: String,
    /// The key that sealed it.
    #[serde(default)]
    pub by: String,
    /// Who made it, where that isn't the key that sealed it: under rules 2, the person's account.
    #[serde(default)]
    pub member: Option<String>,
    /// Whether the room would erase its words now (what someone said, sealed so it can be erased, not erased yet).
    #[serde(default)]
    pub erasable: bool,
    /// For an agent: the person it acts for.
    #[serde(default)]
    pub agent_of: Option<String>,
    pub at: String,
    pub title: String,
    pub body: String,
    pub state: String,
    pub parent: Option<String>,
    #[ts(type = "Record<string, unknown>")]
    pub fields: Value,
    /// The version of the room's rules it was made under.
    #[serde(default)]
    pub charter: String,
    #[serde(default)]
    pub hash: String,
}

#[derive(Serialize, TS, Clone, Debug)]
#[serde(tag = "outcome", rename_all = "snake_case")]
#[ts(export, export_to = "../../src/bindings/")]
pub enum FeedRead {
    /// `from_copy`: the room is unreachable, and this is your own copy of its record.
    Feed { moves: Vec<MoveView>, from_copy: bool },
    Error { message: String },
}

#[derive(Serialize, TS, Clone, Debug)]
#[serde(tag = "outcome", rename_all = "snake_case")]
#[ts(export, export_to = "../../src/bindings/")]
pub enum CallOutcome {
    Ok { text: String },
    Error { message: String },
}

#[derive(Serialize, TS, Clone, Debug)]
#[serde(tag = "outcome", rename_all = "snake_case")]
#[ts(export, export_to = "../../src/bindings/")]
pub enum JoinOutcome {
    /// In. `rules_match`: the room's rules are the ones the invite showed.
    Joined { id: String, rules_match: bool },
    Denied,
    Missing,
    Error { message: String },
}

#[derive(Serialize, TS, Clone, Debug)]
#[serde(tag = "outcome", rename_all = "snake_case")]
#[ts(export, export_to = "../../src/bindings/")]
pub enum HostOutcome {
    Hosted { id: String },
    Error { message: String },
}

#[derive(Deserialize, TS, Clone, Debug)]
#[ts(export, export_to = "../../src/bindings/")]
pub struct HostSpaceInput {
    pub title: String,
    pub kind: String,
    pub charter: String,
    /// Whether anyone may knock with a note, without an invite.
    pub open_door: bool,
}

/// How you'll appear in a room you're about to knock on.
#[derive(Deserialize, TS, Clone, Debug)]
#[serde(tag = "as", rename_all = "snake_case")]
#[ts(export, export_to = "../../src/bindings/")]
pub enum AppearAs {
    /// Your usual self.
    Usual,
    /// A fresh persona, just for this room.
    Fresh { name: String },
}

/// What an invite shows before you knock.
#[derive(Serialize, TS, Clone, Debug)]
#[ts(export, export_to = "../../src/bindings/")]
pub struct DoorView {
    /// The name a knock as your usual self sends, exactly.
    pub usual_name: String,
    pub title: String,
    pub kind: String,
    pub host_name: String,
    pub host: ProviderView,
    pub charter: String,
    pub verbs: Vec<VerbView>,
    /// Whether it carries a secret; without one, a knock needs a note.
    pub invited: bool,
    /// You already have a persona here, or you host it.
    pub already_in: bool,
    /// Everything a knock here sends: the page lists each field, and nothing else goes.
    pub sends: KnockSends,
    /// On a profile's door: what its owner put there for anyone with the link. Theirs to say; nothing checks it.
    pub card: Option<crate::card::DoorCard>,
}

/// What a knock at one door carries, field by field (`spaces::KNOCK_FIELDS`).
#[derive(Serialize, TS, Clone, Debug)]
#[ts(export, export_to = "../../src/bindings/")]
pub struct KnockSends {
    /// Every field, in the order the page lists them.
    pub fields: Vec<String>,
    /// The room's id, as the knock names it.
    pub room: String,
    /// Whether it carries a mark of the invite's key (never the key itself).
    pub invite_mark: bool,
    /// As your usual self.
    pub usual: KnockAs,
}

/// Who a knock says you are, as one of your names.
#[derive(Serialize, TS, Clone, Debug)]
#[ts(export, export_to = "../../src/bindings/")]
pub struct KnockAs {
    /// The name, exactly.
    pub name: String,
    /// The key's mark in this room: the knock carries the whole key.
    pub key_mark: String,
    /// What the account's proof shows: when it was made, its device list's number, and how many devices it names.
    pub account_made: String,
    pub list_number: u64,
    pub devices: u32,
}

#[derive(Serialize, TS, Clone, Debug)]
#[ts(export, export_to = "../../src/bindings/")]
pub struct VerbView {
    pub name: String,
    pub does: String,
}

/// One of your personas, and the rooms you're it in.
#[derive(Serialize, TS, Clone, Debug)]
#[ts(export, export_to = "../../src/bindings/")]
pub struct PersonaView {
    pub id: String,
    pub name: String,
    pub usual: bool,
    /// The rooms you're this name in, by id: the screen names them.
    pub rooms: Vec<String>,
}

/// Your card as you keep it, and where your profile room stands.
#[derive(Serialize, TS, Clone, Debug)]
#[ts(export, export_to = "../../src/bindings/")]
pub struct ProfileCardView {
    pub card: crate::card::Card,
    /// Your profile room, if you have one yet.
    pub profile: Option<String>,
    /// When the card the people you let in see was set, if one is up.
    pub in_room: Option<String>,
    /// Read back from your profile room, since this folder keeps none: every part shows as for the people you let in.
    pub from_room: bool,
}

#[derive(Serialize, TS, Clone, Debug)]
#[serde(tag = "outcome", rename_all = "snake_case")]
#[ts(export, export_to = "../../src/bindings/")]
pub enum ProfileCardSaved {
    /// Kept here. `posted`: a new card went to your profile room. `took_back`: earlier cards whose words were erased.
    Saved { posted: bool, #[ts(type = "number")] took_back: u32 },
    Error { message: String },
}

#[derive(Serialize, TS, Clone, Debug)]
#[ts(export, export_to = "../../src/bindings/")]
pub struct InviteView {
    pub text: String,
}

/// Someone at the door of a room you host.
#[derive(Serialize, TS, Clone, Debug)]
#[ts(export, export_to = "../../src/bindings/")]
pub struct KnockView {
    #[ts(type = "number")]
    pub knock_id: u64,
    pub space: String,
    pub space_title: String,
    /// Attested: the provider observed it.
    pub address: String,
    /// Asserted, from what they wrote: the name they'll go by, their note.
    pub name: String,
    pub note: String,
    pub listed: bool,
    /// Whether they came with this room's current invite, or knocked on an open door.
    pub invited: bool,
    /// Whether the knock checks: signed by the key it names, for this room, lately.
    /// Nothing else it says counts unless it does.
    pub checked: bool,
    /// A member who vouches for them, if any.
    pub vouch: Option<VouchView>,
    pub at: String,
    /// When the door answered it from the room's own record, without a
    /// card: what you did before decides.
    pub answered: Option<KnockAnswered>,
}

/// A knock the door answered from the room's record.
#[derive(Serialize, TS, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
#[ts(export, export_to = "../../src/bindings/")]
pub enum KnockAnswered {
    /// Let in before and still in: let back in, with nothing new admitted.
    LetBackIn,
    /// Removed before: turned away.
    TurnedAway,
}

/// A room another room vouches for: its host minted this invite for the list.
#[derive(Serialize, Deserialize, TS, Clone, Debug)]
#[ts(export, export_to = "../../src/bindings/")]
pub struct DoorwayView {
    pub title: String,
    pub invite: String,
    pub by: String,
    pub at: String,
}

/// Someone a room you host let in, from its record: listed or not.
#[derive(Serialize, TS, Clone, Debug)]
#[ts(export, export_to = "../../src/bindings/")]
pub struct AdmittedView {
    pub name: String,
    pub key: String,
    pub listed: bool,
    pub is_agent: bool,
    /// One of your own agents: its place is your allowances, not the list of people you let in.
    pub yours: bool,
    /// A mark of their key in this room, when someone else let in goes by
    /// the same name (see [`crate::marks`]).
    pub mark: Option<String>,
}

/// Where one ask went, and what the room said.
#[derive(Serialize, TS, Clone, Debug)]
#[ts(export, export_to = "../../src/bindings/")]
pub struct AskSent {
    pub room: String,
    pub outcome: CallOutcome,
}

/// How many moves an agent may make in a room each day without asking you.
#[derive(Serialize, TS, Clone, Debug)]
#[ts(export, export_to = "../../src/bindings/")]
pub struct AllowanceView {
    /// Per kind of move: how many a day without asking. You set these.
    #[ts(type = "Record<string, number>")]
    pub per_day: std::collections::HashMap<crate::door::Reach, u32>,
    #[ts(type = "Record<string, number>")]
    pub used_today: std::collections::HashMap<crate::door::Reach, u32>,
}

/// A member's seal on a newcomer's key.
#[derive(Serialize, TS, Clone, Debug)]
#[ts(export, export_to = "../../src/bindings/")]
pub struct VouchView {
    pub by: String,
    /// Whether the voucher is a member of the room being knocked on.
    pub member_here: bool,
    /// Whether the seal holds and names this newcomer's key.
    pub holds: bool,
}

#[derive(Serialize, TS, Clone, Debug)]
#[serde(tag = "event", rename_all = "snake_case")]
#[ts(export, export_to = "../../src/bindings/")]
pub enum SpaceEvent {
    Updated { uri: String },
    End,
}

impl From<rmcp::model::ServerNotification> for SpaceEvent {
    fn from(n: rmcp::model::ServerNotification) -> Self {
        use rmcp::model::ServerNotification;
        match n {
            ServerNotification::ResourceUpdatedNotification(n) => SpaceEvent::Updated { uri: n.params.uri },
            ServerNotification::ResourceListChangedNotification(_) | ServerNotification::ToolListChangedNotification(_) => SpaceEvent::Updated { uri: "*".into() },
            // rmcp's enum, not the SDK's: it grows with the MCP spec, and
            // anything else is not something the screen re-reads for.
            _ => SpaceEvent::Updated { uri: "".into() },
        }
    }
}

#[derive(Serialize, TS, Clone, Debug)]
#[serde(tag = "event", rename_all = "snake_case")]
#[ts(export, export_to = "../../src/bindings/")]
pub enum KnockEvent {
    Knock { knock: KnockView },
    End,
}

// --- home: every Space's moves, together ------------------------------------

#[derive(Serialize, TS, Clone, Debug)]
#[ts(export, export_to = "../../src/bindings/")]
pub struct HomeMove {
    pub space: SpaceSummary,
    pub entry: MoveView,
}

/// A receipt for a task done, attributed to the Space that issued it.
/// Never merged, ranked or converted.
#[derive(Serialize, TS, Clone, Debug)]
#[ts(export, export_to = "../../src/bindings/")]
pub struct ReceiptView {
    pub title: String,
    pub for_title: String,
    /// The room that issued it, as the receipt itself names it.
    pub room_title: String,
    /// That room, when you're in it: somewhere to open.
    pub space: Option<SpaceSummary>,
    pub to: String,
    pub at: String,
    /// The name the room's host gave, as the receipt says.
    pub issued_by: String,
    /// Whether whoever sealed it is someone you've met in a room you're in.
    pub known: bool,
    /// Whether its seal holds and was made by the host of the room it names.
    pub holds: bool,
    /// Which of your names earned it.
    pub earned_as: String,
    /// Whether that's your usual name: only then can it go on your profile
    /// without linking your names.
    pub earned_as_usual: bool,
    /// The sealed receipt itself, to pin to your profile.
    #[ts(type = "unknown")]
    pub statement: Value,
}

#[derive(Serialize, TS, Clone, Debug)]
#[ts(export, export_to = "../../src/bindings/")]
pub struct ProfileView {
    pub receipts: Vec<ReceiptView>,
    pub shows: Vec<MoveView>,
    pub agents: Vec<AgentView>,
    pub machines: Vec<MachineView>,
    pub volumes: Vec<VolumeView>,
    pub home: Option<String>,
    /// Your profile room, which anyone with its link can knock on.
    pub profile: Option<String>,
    /// Whether your profile seals what visitors leave to you: its settings name your notes key.
    pub profile_sealed: bool,
    pub personas: Vec<PersonaView>,
}

/// Someone you share a Space with.
#[derive(Serialize, TS, Clone, Debug)]
#[ts(export, export_to = "../../src/bindings/")]
pub struct PersonView {
    pub name: String,
    pub key: String,
    pub is_agent: bool,
    pub agent_of: Option<String>,
    /// The Spaces you are both in, by id: the same key in each.
    pub spaces: Vec<String>,
}

// --- cards: an agent asks its person -------------------------------------

#[derive(Serialize, Deserialize, TS, Clone, Debug, PartialEq)]
#[serde(rename_all = "snake_case")]
#[ts(export, export_to = "../../src/bindings/")]
pub enum CardKind {
    Question,
    Choice,
    Credential,
}

#[derive(Serialize, Deserialize, TS, Clone, Debug)]
#[ts(export, export_to = "../../src/bindings/")]
pub struct CardView {
    #[ts(type = "number")]
    pub id: u64,
    pub agent: String,
    /// Who is asking, when it isn't the agent: a visitor hiring it, say.
    pub from: Option<String>,
    pub kind: CardKind,
    /// The agent's own words, for a question it asks. Empty for a card about
    /// a move or a hire: the screen words those, from `call` or `hire`.
    pub question: String,
    pub options: Vec<String>,
    pub at: String,
    /// A move the agent wants to make in a room, whole.
    pub call: Option<CardCall>,
    /// A visitor's hire of one of your agents.
    pub hire: Option<CardHire>,
}

/// What an agent wants to do in a room, whole: nothing the card leaves out.
#[derive(Serialize, Deserialize, TS, Clone, Debug)]
#[ts(export, export_to = "../../src/bindings/")]
pub struct CardCall {
    pub room: String,
    pub room_title: String,
    /// A room's verb, or one of the door's own: `space_feed`, `table_read`, `asks_open`…
    pub verb: String,
    pub reach: crate::door::Reach,
    /// Every argument, as the agent sent it.
    #[ts(type = "Record<string, unknown>")]
    pub arguments: Value,
    /// The title of the move it's about, when it names one: a task, an ask.
    pub about: Option<String>,
}

/// A visitor's hire, in their own words: the name is whatever they typed.
#[derive(Serialize, Deserialize, TS, Clone, Debug)]
#[ts(export, export_to = "../../src/bindings/")]
pub struct CardHire {
    pub from: String,
    pub what: String,
    pub pledge: Option<String>,
    /// A mark of their key in that room, when someone else there goes by
    /// the same name (see [`crate::marks`]).
    #[serde(default)]
    pub mark: Option<String>,
}

#[derive(Serialize, TS, Clone, Debug)]
#[serde(tag = "event", rename_all = "snake_case")]
#[ts(export, export_to = "../../src/bindings/")]
pub enum CardEvent {
    Card { card: CardView },
    Answered {
        #[ts(type = "number")]
        id: u64,
    },
    /// The agent stopped waiting: the card is taken back, and an answer to it does nothing.
    Withdrawn {
        #[ts(type = "number")]
        id: u64,
    },
    End,
}

// --- machines (ours until the wire has them) -----------------------------

#[derive(Serialize, TS, Clone, Debug)]
#[ts(export, export_to = "../../src/bindings/")]
pub struct MachineView {
    pub identity: ProviderView,
    /// Its own volumes, from its own listing.
    pub volumes: Vec<VolumeView>,
    /// Why its volumes could not be listed, if they could not.
    pub volumes_problem: Option<String>,
    pub added: String,
    /// What you call it. Yours, kept by the app; the daemon's name stays underneath.
    pub name: Option<String>,
}

/// The key a machine's own name is kept under.
pub fn identity_key(p: &ProviderView) -> String {
    match p {
        ProviderView::Outgoing { address } => format!("outgoing:{address}"),
        ProviderView::IncomingUnbrokered { identity } => format!("incoming:{identity}"),
    }
}

#[derive(Deserialize, TS, Clone, Debug)]
#[serde(tag = "way", rename_all = "snake_case")]
#[ts(export, export_to = "../../src/bindings/")]
pub enum NewMachineInput {
    Dial { address: String, key: String },
    Accept { identity: String, key: String },
}

// --- saved Views ---------------------------------------------------------

#[derive(Deserialize, Serialize, TS, Clone, Debug)]
#[ts(export, export_to = "../../src/bindings/")]
pub struct SavedView {
    pub id: String,
    pub title: String,
    pub query: LogsQuery,
    pub saved: String,
}

// --- tabs (Rust-owned) ---------------------------------------------------

#[derive(Serialize, Deserialize, TS, Clone, Debug, PartialEq)]
#[serde(tag = "kind", rename_all = "snake_case")]
#[ts(export, export_to = "../../src/bindings/")]
pub enum TabKind {
    Agent { name: String },
    NewAgent,
    Storage,
    Machines,
    Views,
    Spaces,
    Space { id: String },
    /// An invite, read before knocking.
    Door { invite: String },
    Home,
    Inbox,
    Profile,
}

#[derive(Serialize, TS, Clone, Debug)]
#[ts(export, export_to = "../../src/bindings/")]
pub struct TabView {
    pub key: String,
    pub tab: TabKind,
}

#[derive(Serialize, TS, Clone, Debug)]
#[ts(export, export_to = "../../src/bindings/")]
pub struct TabsSnapshot {
    #[ts(type = "number")]
    pub generation: u64,
    pub tabs: Vec<TabView>,
    pub focused: Option<String>,
}

// --- the app itself --------------------------------------------------------

/// Where the first-run page stands. Until it's finished there's no you
/// here, and nothing is signed or sent.
#[derive(Serialize, TS, Clone, Debug, PartialEq)]
#[serde(tag = "state", rename_all = "snake_case")]
#[ts(export, export_to = "../../src/bindings/")]
pub enum FirstRunView {
    Done,
    New,
    /// A folder from before accounts: it has you under the name an earlier
    /// version of the app gave you.
    Earlier { name: String },
    /// The page can't be finished in this copy of the app; the words say why.
    Blocked { words: String },
}

impl From<crate::identity::FirstRun> for FirstRunView {
    fn from(f: crate::identity::FirstRun) -> Self {
        match f {
            crate::identity::FirstRun::Done => FirstRunView::Done,
            crate::identity::FirstRun::New => FirstRunView::New,
            crate::identity::FirstRun::Earlier { name } => FirstRunView::Earlier { name },
            crate::identity::FirstRun::Blocked(words) => FirstRunView::Blocked { words: words.into() },
        }
    }
}

#[derive(Serialize, TS, Clone, Debug)]
#[ts(export, export_to = "../../src/bindings/")]
pub struct AppInfo {
    /// True while the daemon is the stand-in.
    pub stand_in: bool,
    /// Whether anything answers for the daemon, machines and rooms. A build
    /// without the stand-in has nothing there yet, and the screens say so.
    pub network: bool,
    /// The commit of the SDK's branch the seam was built against.
    pub contract_pin: String,
    /// Where the stand-in keeps its host's files.
    pub stand_in_host: Option<String>,
    /// The folder the app keeps its files in.
    pub folder: String,
    /// Whether this copy of the app holds that folder. One that doesn't
    /// changes nothing in it, and the screens say so.
    pub folder_held: FolderHeld,
}

#[derive(Serialize, TS, Clone, Debug, PartialEq)]
#[serde(tag = "state", rename_all = "snake_case")]
#[ts(export, export_to = "../../src/bindings/")]
pub enum FolderHeld {
    /// This copy holds it, for as long as it runs.
    Yes,
    /// Another copy of the app holds it.
    InUse,
    /// The system couldn't say whether another copy does, in its own words.
    Unchecked { error: String },
}

impl From<&crate::store::Hold> for FolderHeld {
    fn from(h: &crate::store::Hold) -> Self {
        match h {
            crate::store::Hold::Held(_) => FolderHeld::Yes,
            crate::store::Hold::Elsewhere => FolderHeld::InUse,
            crate::store::Hold::Unchecked(error) => FolderHeld::Unchecked { error: error.clone() },
        }
    }
}

/// Your keys file, when it can't be used: nothing is signed, and it is left as it is.
#[derive(Serialize, TS, Clone, Debug, PartialEq)]
#[ts(export, export_to = "../../src/bindings/")]
pub struct KeysBrokenView {
    pub file: String,
    /// A newer version of the app wrote it (otherwise, it can't be read).
    pub newer: bool,
}

/// A file the app couldn't use since it started, and what it did about it.
#[derive(Serialize, TS, Clone, Debug, PartialEq)]
#[ts(export, export_to = "../../src/bindings/")]
pub struct FileNoticeView {
    /// What kind of file it is: `keys`, `views`, `record copy`… The
    /// screen words it from `src/strings.ts`.
    pub kind: String,
    /// For your copy of a room's record: the room's title, when the app knows the room.
    pub room: Option<String>,
    /// Whether it's the last good copy kept beside the file, not the file itself.
    pub last_good_copy: bool,
    /// Where it was.
    pub file: String,
    pub why: FileWhy,
    /// Where it is now, when it was set aside; none when it was left where it is.
    pub kept_as: Option<String>,
    /// What the system said, when it wouldn't read the file.
    pub error: Option<String>,
    pub carried_on: CarriedOnView,
}

#[derive(Serialize, TS, Clone, Debug, PartialEq)]
#[serde(rename_all = "snake_case")]
#[ts(export, export_to = "../../src/bindings/")]
pub enum FileWhy {
    /// It won't parse, or it's another kind of file.
    Damaged,
    /// It parses, but what it holds doesn't check: a room's record that isn't that room's, or doesn't replay.
    Refused,
    /// A newer version of the app wrote it.
    Newer,
    /// The system wouldn't open or read it this time.
    Unread,
}

#[derive(Serialize, TS, Clone, Debug, PartialEq)]
#[serde(rename_all = "snake_case")]
#[ts(export, export_to = "../../src/bindings/")]
pub enum CarriedOnView {
    /// The last good copy, kept beside it.
    LastGood,
    /// Nothing: the app started that file empty.
    Empty,
    /// What the app had open: the file went bad after it was read.
    WhatItHad,
    /// Nothing, and nothing is written there until the app starts again.
    NothingThisLaunch,
}

impl FileNoticeView {
    /// A notice as the screen shows it; `room` names the room, for a record copy.
    pub fn of(n: &crate::store::Notice, room: Option<String>) -> Self {
        use crate::store::{CarriedOn, Done, Why};
        let (why, kept_as, error) = match &n.done {
            Done::SetAside { kept_as, why } => (
                match why {
                    Why::Damaged => FileWhy::Damaged,
                    Why::Refused => FileWhy::Refused,
                    Why::Newer(_) => FileWhy::Newer,
                },
                Some(kept_as.display().to_string()),
                None,
            ),
            Done::LeftInPlace { error } => (FileWhy::Unread, None, Some(error.clone())),
        };
        FileNoticeView {
            kind: n.kind.to_owned(),
            room,
            last_good_copy: n.last_good_copy(),
            file: n.file.display().to_string(),
            why,
            kept_as,
            error,
            carried_on: match n.carried_on {
                CarriedOn::LastGood => CarriedOnView::LastGood,
                CarriedOn::Empty => CarriedOnView::Empty,
                CarriedOn::WhatItHad => CarriedOnView::WhatItHad,
                CarriedOn::NothingThisLaunch => CarriedOnView::NothingThisLaunch,
            },
        }
    }
}

/// One entry in the action registry: everything a person can do here,
/// by name — the list an agent's door will expose.
#[derive(Serialize, TS, Clone, Debug)]
#[ts(export, export_to = "../../src/bindings/")]
pub struct ActionInfo {
    pub name: String,
    pub does: String,
}

#[allow(dead_code)]
const _: &str = OUT;

// --- the door served on this machine, for local agents ---------------------

/// A local agent you already run yourself, reaching the door on this
/// machine's loopback with a token of its own.
#[derive(Serialize, TS, Clone, Debug, PartialEq)]
#[ts(export, export_to = "../../src/bindings/")]
pub struct LocalAgentView {
    pub id: String,
    /// What you call it.
    pub name: String,
    /// Its slot, as allowances and cards name it.
    pub slot: String,
    /// The exact line that adds it to Claude Code. It names the helper
    /// that prints the token, never the token itself.
    pub connect_line: Option<String>,
    pub added: String,
}

/// Whether the door is listening on this machine, and where.
#[derive(Serialize, TS, Clone, Debug, PartialEq)]
#[ts(export, export_to = "../../src/bindings/")]
pub struct DoorStatusView {
    /// The port the door was given once. It never moves on its own.
    pub port: Option<u16>,
    pub state: DoorState,
}

#[derive(Serialize, TS, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
#[ts(export, export_to = "../../src/bindings/")]
pub enum DoorState {
    /// No local agent is added, so nothing listens.
    NoAgents,
    /// Listening on 127.0.0.1, on its port.
    Listening,
    /// Something else holds its port, so the door isn't listening.
    PortTaken,
    /// This copy of the app doesn't hold its folder, so it serves nothing.
    NotThisCopy,
}
