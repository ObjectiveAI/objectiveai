//! What the screen is handed: this app's own types, not the daemon's.
//!
//! Every type here writes its own TypeScript (`cargo test` exports them to
//! `src/bindings/`). Every conversion from a daemon type is an exhaustive
//! `match` with no catch-all, so a variant Ronald adds fails this build
//! instead of drifting past the screen.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use ts_rs::TS;

use diverge_daemon_sdk::endpoints::agents;
use diverge_daemon_sdk::endpoints::agents::logs::server::response::{Identity, Item, ItemWrapper};
use diverge_provider_sdk::endpoints::containers::agents::run::server::response::AgenticLoopChunk;
use diverge_provider_sdk::shared::error::Error as WireError;
use diverge_provider_sdk::shared::filetree::response::{Frame as TreeFrame, Node};

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
    /// JSON Schema of the image's settings, from Ronald's source.
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
    pub container_path: String,
}

#[derive(Deserialize, TS, Clone, Debug)]
#[ts(export, export_to = "../../src/bindings/")]
pub struct FuseMountInput {
    pub daemon_path: String,
    pub container_path: String,
}

fn components(path: &str) -> Vec<String> {
    path.split('/').filter(|p| !p.is_empty()).map(str::to_owned).collect()
}

impl CreateAgentInput {
    /// Into the daemon's own create. Every field named, so a field Ronald
    /// adds fails this build.
    pub fn into_request(self) -> Result<agents::create::client::request::Frame, String> {
        use agents::create::client::request::{Frame, FuseMount, Image, Provider, VolumeMount};
        let kind = crate::catalog::ALL
            .into_iter()
            .find(|k| k.key() == self.image_kind)
            .ok_or_else(|| format!("no image called {}", self.image_kind))?;
        let fuse = |mounts: Vec<FuseMountInput>| {
            mounts
                .into_iter()
                .map(|m| FuseMount { daemon_path: m.daemon_path, container_path: components(&m.container_path) })
                .collect::<Vec<_>>()
        };
        Ok(Frame {
            image: Image { name: kind.image_name(), digest: crate::catalog::UNBUILT_DIGEST.into() },
            memory: self.memory,
            disk: self.disk,
            provider: self.provider.as_ref().map(|p| Provider {
                identity: p.into(),
                volume_mounts: self
                    .volume_mounts
                    .iter()
                    .map(|m| VolumeMount {
                        volume_name: m.volume_name.clone(),
                        volume_relative_path: components(&m.volume_relative_path),
                        container_path: components(&m.container_path),
                    })
                    .collect(),
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

// --- files ---------------------------------------------------------------

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
        watched: bool,
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
            Node::Directory { name, created_at: _, modified_at: _, changes, children } => {
                FileNode::Directory { name: name.clone(), watched: *changes, children: children.iter().map(Into::into).collect() }
            }
            Node::Symlink { name, path, created_at: _, modified_at: _ } => FileNode::Symlink { name: name.clone(), target: path.clone() },
        }
    }
}

#[derive(Serialize, TS, Clone, Debug)]
#[serde(tag = "event", rename_all = "snake_case")]
#[ts(export, export_to = "../../src/bindings/")]
pub enum TreeEvent {
    Snapshot { children: Vec<FileNode> },
    Inserted { path: Vec<String>, node: FileNode },
    Modified { path: Vec<String>, node: FileNode },
    Removed { path: Vec<String> },
    Error { message: String },
    End,
}

impl From<diverge_daemon_sdk::endpoints::filesystem::filetree::server::response::Frame> for TreeEvent {
    fn from(frame: diverge_daemon_sdk::endpoints::filesystem::filetree::server::response::Frame) -> Self {
        use diverge_daemon_sdk::endpoints::filesystem::filetree::server::response::Frame;
        match frame {
            Frame::Filetree(tree) => match tree {
                TreeFrame::Snapshot { children } => TreeEvent::Snapshot { children: children.iter().map(Into::into).collect() },
                TreeFrame::Inserted { path, node } => TreeEvent::Inserted { path, node: (&node).into() },
                TreeFrame::Modified { path, node } => TreeEvent::Modified { path, node: (&node).into() },
                TreeFrame::Removed { path } => TreeEvent::Removed { path },
            },
            Frame::Error(error) => TreeEvent::Error { message: error_text(&error) },
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

impl From<diverge_daemon_sdk::endpoints::filesystem::write::server::response::Frame> for FileWritten {
    fn from(frame: diverge_daemon_sdk::endpoints::filesystem::write::server::response::Frame) -> Self {
        use diverge_daemon_sdk::endpoints::filesystem::write::server::response::Frame;
        match frame {
            Frame::Written(_) => FileWritten::Written,
            Frame::Error(error) => FileWritten::Error { message: error_text(&error) },
        }
    }
}

// --- machines (ours until the wire has them) -----------------------------

#[derive(Serialize, TS, Clone, Debug)]
#[ts(export, export_to = "../../src/bindings/")]
pub struct MachineView {
    pub identity: ProviderView,
    pub volumes: Vec<String>,
    pub added: String,
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
    Files,
    Machines,
    Views,
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

#[derive(Serialize, TS, Clone, Debug)]
#[ts(export, export_to = "../../src/bindings/")]
pub struct AppInfo {
    /// True while the daemon is the stand-in.
    pub stand_in: bool,
    /// The commit of Ronald's branch the seam was built against.
    pub contract_pin: String,
    /// Where the stand-in keeps its host's files.
    pub stand_in_host: Option<String>,
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
