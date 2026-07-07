//! Materialized consumer of the cli daemon's `/agents/instances/{*aih}`
//! endpoint — one agent's full conversation, history + live.
//!
//! [`WebSocketAgentsInstancesListener`] connects once, then folds every
//! incoming [`AgentInstanceEvent`] into an in-memory conversation: the
//! DB snapshot replays as `Row` events, `Live` marks the seam, then
//! live rows stream as the agent produces them. Rows are keyed
//! full-value upserts — a re-sent identity REPLACES the prior row
//! (later = more complete), which also converges any snapshot/live
//! overlap.
//!
//! The coalescer is the incremental analogue of the CLI's
//! `read_all` block builder: consecutive rows sharing the boundary
//! tuple `(class, agent_instance_hierarchy, response_id)` (+ sender /
//! queue id for notifications, + `tool_call_id` for the two tool
//! classes) join the LAST block; anything else opens a new one. A
//! re-sent row never re-runs the boundary test — its identity routes
//! it straight back to its block. Blocks materialize as
//! [`ConversationBlock`]s in conversation order.
//!
//! Three ways to observe, mirroring
//! [`super::super::websocket_agents_instances_list_listener`]:
//! [`conversation`](WebSocketAgentsInstancesListener::conversation),
//! an on-change callback, and
//! [`subscribe`](WebSocketAgentsInstancesListener::subscribe).
//!
//! One listener = one connection: the view updates until the socket
//! closes (the daemon disconnects lagging clients rather than dropping
//! frames), then freezes. Reconnection is the caller's loop — build a
//! new listener; the fresh snapshot replaces everything. The write
//! half of the socket is retained for the planned client→daemon
//! message requests over this stream.

use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;

use futures::stream::{SplitSink, SplitStream};
use futures::{SinkExt, StreamExt};
use tokio::sync::{Mutex, watch};
use tokio_tungstenite::tungstenite;
use tokio_tungstenite::tungstenite::client::IntoClientRequest;

use super::{
    AgentInstanceEvent, ConversationBlock, ConversationChoice, ConversationRow, RowContent,
    RowTableKind,
};
use crate::cli::command::command_executor::websocket::AuthEnvelope;
use crate::cli::websocket_agents_instances_list_listener::AgentRecord;

/// The conversation on-change callback: invoked with the full current
/// conversation (blocks in conversation order) after each applied
/// conversation event.
pub type OnChange = Box<dyn Fn(&[ConversationBlock]) + Send + Sync>;

/// The agent-status on-change callback: invoked with the agent's
/// refreshed list record after each applied `Agent` event. Structurally
/// independent of the conversation callback — neither fires for the
/// other's events.
pub type OnAgentChange = Box<dyn Fn(&AgentRecord) + Send + Sync>;

type Ws = tokio_tungstenite::WebSocketStream<
    tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
>;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// The URL failed to build into a client upgrade request, or the
    /// connection/upgrade itself failed.
    #[error("connect daemon agent-instance websocket: {0}")]
    Connect(tungstenite::Error),
    /// The established connection failed mid-stream.
    #[error("daemon agent-instance websocket: {0}")]
    Ws(tungstenite::Error),
}

/// Coarse block-class for a [`RowTableKind`] — mirrors the CLI
/// `read_all` classifier (head kinds map to the class of the block
/// they carry metadata for).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BlockClass {
    ClientNotification,
    AssistantResponse,
    ToolResponse,
    RequestMessageUser,
    RequestMessageAssistant,
    RequestMessageTool,
    VectorRequestChoices,
    VectorResponseVote,
}

fn block_class(t: RowTableKind) -> BlockClass {
    use RowTableKind as T;
    match t {
        T::MessageQueueText
        | T::MessageQueueImage
        | T::MessageQueueAudio
        | T::MessageQueueVideo
        | T::MessageQueueFile => BlockClass::ClientNotification,
        T::AssistantResponseRefusal
        | T::AssistantResponseReasoning
        | T::AssistantResponseToolCalls
        | T::AssistantResponseContentText
        | T::AssistantResponseContentImage
        | T::AssistantResponseContentAudio
        | T::AssistantResponseContentVideo
        | T::AssistantResponseContentFile => BlockClass::AssistantResponse,
        T::ToolResponse
        | T::ToolResponseContentText
        | T::ToolResponseContentImage
        | T::ToolResponseContentAudio
        | T::ToolResponseContentVideo
        | T::ToolResponseContentFile => BlockClass::ToolResponse,
        T::RequestMessageUserContentText
        | T::RequestMessageUserContentImage
        | T::RequestMessageUserContentAudio
        | T::RequestMessageUserContentVideo
        | T::RequestMessageUserContentFile => BlockClass::RequestMessageUser,
        T::RequestMessageAssistantRefusal
        | T::RequestMessageAssistantReasoning
        | T::RequestMessageAssistantToolCalls
        | T::RequestMessageAssistantContentText
        | T::RequestMessageAssistantContentImage
        | T::RequestMessageAssistantContentAudio
        | T::RequestMessageAssistantContentVideo
        | T::RequestMessageAssistantContentFile => BlockClass::RequestMessageAssistant,
        T::RequestMessageTool
        | T::RequestMessageToolContentText
        | T::RequestMessageToolContentImage
        | T::RequestMessageToolContentAudio
        | T::RequestMessageToolContentVideo
        | T::RequestMessageToolContentFile => BlockClass::RequestMessageTool,
        T::RequestVectorChoice
        | T::RequestVectorChoiceContentText
        | T::RequestVectorChoiceContentImage
        | T::RequestVectorChoiceContentAudio
        | T::RequestVectorChoiceContentVideo
        | T::RequestVectorChoiceContentFile => BlockClass::VectorRequestChoices,
        T::ResponseVectorVote => BlockClass::VectorResponseVote,
    }
}

/// A row's replace-at key: identical between the snapshot replay and
/// the live tee (AIH is constant per connection and omitted).
type RowIdentity = (RowTableKind, String, i64, Option<i64>);

/// One in-progress block. Parts are ordered by
/// `(row_index, row_sub_index, table)` — arrival-independent, so a
/// late replacement lands in place.
struct BlockState {
    class: BlockClass,
    agent_instance_hierarchy: String,
    response_id: String,
    /// Tool classes: from the HEAD row (live) or content-row joins
    /// (snapshot), whichever arrives.
    tool_call_id: Option<String>,
    sender: Option<String>,
    message_queue_id: Option<i64>,
    queued_at: Option<String>,
    key: Option<String>,
    /// VectorResponseVote's inline value.
    vote: Option<Vec<rust_decimal::Decimal>>,
    /// VectorRequestChoices: voting key per choice index.
    choice_keys: HashMap<i64, String>,
    parts: BTreeMap<(i64, Option<i64>, RowTableKind), ConversationRow>,
}

impl BlockState {
    fn new(class: BlockClass, row: &ConversationRow) -> Self {
        Self {
            class,
            agent_instance_hierarchy: row.agent_instance_hierarchy.clone(),
            response_id: row.response_id.clone(),
            tool_call_id: None,
            sender: None,
            message_queue_id: None,
            queued_at: None,
            key: None,
            vote: None,
            choice_keys: HashMap::new(),
            parts: BTreeMap::new(),
        }
    }

    /// Does `row` (a NEW identity of `class`) belong to this block?
    /// The read_all boundary tuple, evaluated against the LAST block:
    /// `(class, aih, response_id)` + sender/queue-id for notifications
    /// + `tool_call_id` for the tool classes. A live tool CONTENT row
    /// carries no `tool_call_id` (only its head does) — adjacency
    /// stands in, which is exact because the writer emits head then
    /// contents consecutively.
    fn accepts(&self, class: BlockClass, row: &ConversationRow) -> bool {
        if self.class != class
            || self.agent_instance_hierarchy != row.agent_instance_hierarchy
            || self.response_id != row.response_id
        {
            return false;
        }
        match class {
            BlockClass::ClientNotification => {
                self.sender == row.sender_agent_instance_hierarchy
                    && self.message_queue_id == row.message_queue_id
            }
            BlockClass::ToolResponse | BlockClass::RequestMessageTool => {
                match (&row.tool_call_id, &self.tool_call_id) {
                    (Some(row_id), Some(block_id)) => row_id == block_id,
                    // No id on one side: adjacency decides.
                    _ => true,
                }
            }
            _ => true,
        }
    }

    /// Fold one row in: metadata always merges; HEAD/vote rows carry
    /// no part; everything else upserts its part slot.
    fn apply(&mut self, row: ConversationRow) {
        if row.tool_call_id.is_some() {
            self.tool_call_id = row.tool_call_id.clone();
        }
        if let Some(key) = &row.choice_key {
            self.choice_keys.insert(row.row_index, key.clone());
        }
        if row.sender_agent_instance_hierarchy.is_some() {
            self.sender = row.sender_agent_instance_hierarchy.clone();
        }
        if row.queued_at.is_some() {
            self.queued_at = row.queued_at.clone();
        }
        if row.message_queue_key.is_some() {
            self.key = row.message_queue_key.clone();
        }
        if row.message_queue_id.is_some() {
            self.message_queue_id = row.message_queue_id;
        }
        match &row.content {
            RowContent::Head => {}
            RowContent::Vote { vote } => self.vote = Some(vote.clone()),
            _ => {
                self.parts
                    .insert((row.row_index, row.row_sub_index, row.table), row);
            }
        }
    }

    /// Materialize — `None` while the block has nothing presentable
    /// yet (e.g. a lone head row whose contents are in flight).
    fn to_block(&self) -> Option<ConversationBlock> {
        let parts = || self.parts.values().cloned().collect::<Vec<_>>();
        Some(match self.class {
            BlockClass::RequestMessageUser => {
                if self.parts.is_empty() {
                    return None;
                }
                ConversationBlock::RequestMessageUser {
                    agent_instance_hierarchy: self.agent_instance_hierarchy.clone(),
                    response_id: self.response_id.clone(),
                    parts: parts(),
                }
            }
            BlockClass::RequestMessageAssistant => {
                if self.parts.is_empty() {
                    return None;
                }
                ConversationBlock::RequestMessageAssistant {
                    agent_instance_hierarchy: self.agent_instance_hierarchy.clone(),
                    response_id: self.response_id.clone(),
                    parts: parts(),
                }
            }
            BlockClass::AssistantResponse => {
                if self.parts.is_empty() {
                    return None;
                }
                ConversationBlock::AssistantResponse {
                    agent_instance_hierarchy: self.agent_instance_hierarchy.clone(),
                    response_id: self.response_id.clone(),
                    parts: parts(),
                }
            }
            BlockClass::RequestMessageTool => {
                if self.parts.is_empty() {
                    return None;
                }
                ConversationBlock::RequestMessageTool {
                    agent_instance_hierarchy: self.agent_instance_hierarchy.clone(),
                    response_id: self.response_id.clone(),
                    tool_call_id: self.tool_call_id.clone().unwrap_or_default(),
                    parts: parts(),
                }
            }
            BlockClass::ToolResponse => {
                if self.parts.is_empty() {
                    return None;
                }
                ConversationBlock::ToolResponse {
                    agent_instance_hierarchy: self.agent_instance_hierarchy.clone(),
                    response_id: self.response_id.clone(),
                    tool_call_id: self.tool_call_id.clone().unwrap_or_default(),
                    parts: parts(),
                }
            }
            BlockClass::VectorRequestChoices => {
                if self.parts.is_empty() {
                    return None;
                }
                // Parts iterate ordered by (choice index, part index);
                // group consecutive runs of one choice index.
                let mut choices: Vec<ConversationChoice> = Vec::new();
                let mut current: Option<(i64, ConversationChoice)> = None;
                for row in self.parts.values() {
                    match &mut current {
                        Some((index, choice)) if *index == row.row_index => {
                            choice.parts.push(row.clone());
                        }
                        _ => {
                            if let Some((_, done)) = current.take() {
                                choices.push(done);
                            }
                            current = Some((
                                row.row_index,
                                ConversationChoice {
                                    key: self
                                        .choice_keys
                                        .get(&row.row_index)
                                        .cloned()
                                        .unwrap_or_default(),
                                    parts: vec![row.clone()],
                                },
                            ));
                        }
                    }
                }
                if let Some((_, done)) = current.take() {
                    choices.push(done);
                }
                ConversationBlock::VectorRequestChoices {
                    agent_instance_hierarchy: self.agent_instance_hierarchy.clone(),
                    response_id: self.response_id.clone(),
                    choices,
                }
            }
            BlockClass::VectorResponseVote => ConversationBlock::VectorResponseVote {
                agent_instance_hierarchy: self.agent_instance_hierarchy.clone(),
                response_id: self.response_id.clone(),
                vote: self.vote.clone()?,
            },
            BlockClass::ClientNotification => {
                if self.parts.is_empty() {
                    return None;
                }
                ConversationBlock::ClientNotification {
                    agent_instance_hierarchy: self.agent_instance_hierarchy.clone(),
                    response_id: self.response_id.clone(),
                    sender_agent_instance_hierarchy: self.sender.clone(),
                    queued_at: self.queued_at.clone(),
                    key: self.key.clone(),
                    parts: parts(),
                }
            }
        })
    }
}

/// The materialized conversation, folded incrementally.
#[derive(Default)]
struct ConversationState {
    /// Arrival order == conversation order (snapshot `"index"` order,
    /// then live write order).
    blocks: Vec<BlockState>,
    /// Replace-at-key routing: a re-sent row goes straight back to its
    /// block, never through the boundary test.
    row_to_block: HashMap<RowIdentity, usize>,
    live: bool,
}

impl ConversationState {
    fn apply(&mut self, event: AgentInstanceEvent) {
        match event {
            AgentInstanceEvent::Row { row } => self.apply_row(row),
            AgentInstanceEvent::Live => self.live = true,
            // Agent-status events never reach the conversation state —
            // the pump routes them to their own slot.
            AgentInstanceEvent::Agent { .. } => {}
        }
    }

    fn apply_row(&mut self, row: ConversationRow) {
        let identity: RowIdentity = (
            row.table,
            row.response_id.clone(),
            row.row_index,
            row.row_sub_index,
        );
        let target = if let Some(&index) = self.row_to_block.get(&identity) {
            index
        } else {
            let class = block_class(row.table);
            let index = match self.blocks.last() {
                Some(last) if last.accepts(class, &row) => self.blocks.len() - 1,
                _ => {
                    self.blocks.push(BlockState::new(class, &row));
                    self.blocks.len() - 1
                }
            };
            self.row_to_block.insert(identity, index);
            index
        };
        self.blocks[target].apply(row);
    }

    fn conversation(&self) -> Vec<ConversationBlock> {
        self.blocks.iter().filter_map(BlockState::to_block).collect()
    }
}

/// The shared inner state, held by the listener handle and its pump.
struct Shared {
    state: Mutex<ConversationState>,
    /// The agent's list record (active/tags/counters) — structurally
    /// separate from the conversation. `None` until the first `Agent`
    /// event lands (the daemon ships one right after auth).
    agent: Mutex<Option<AgentRecord>>,
    /// Bumped per applied event (conversation OR agent); wakes every
    /// [`subscribe`](WebSocketAgentsInstancesListener::subscribe) waiter.
    changes: watch::Sender<u64>,
    on_change: Option<OnChange>,
    on_agent_change: Option<OnAgentChange>,
    /// The connection's write half — retained for the planned
    /// client→daemon message requests over this stream; unused today.
    #[allow(dead_code)]
    sink: Mutex<SplitSink<Ws, tungstenite::Message>>,
}

/// Unconnected configuration — [`WebSocketAgentsInstancesListener::new`] +
/// builder methods + [`WebSocketAgentsInstancesListenerBuilder::connect`].
pub struct WebSocketAgentsInstancesListenerBuilder {
    /// Full connect URL: the daemon's published base address +
    /// `/agents/instances/` + the agent's hierarchy, e.g.
    /// `ws://127.0.0.1:49152/agents/instances/root/child-abc`.
    url: String,
    signature: Option<String>,
    on_change: Option<OnChange>,
    on_agent_change: Option<OnAgentChange>,
}

impl WebSocketAgentsInstancesListenerBuilder {
    /// Attach the daemon auth signature (the pre-derived
    /// `sha256=<hex(SHA256(DAEMON_SECRET))>`), sent verbatim in the
    /// [`AuthEnvelope`] preamble. Without it the daemon must be running
    /// without a secret.
    pub fn signature(mut self, signature: impl Into<String>) -> Self {
        self.signature = Some(signature.into());
        self
    }

    /// Register a callback invoked with the full current conversation
    /// after every applied CONVERSATION event (never for agent-status
    /// events). Runs on the pump task — keep it cheap; for state on
    /// demand use
    /// [`conversation`](WebSocketAgentsInstancesListener::conversation).
    pub fn on_change(
        mut self,
        callback: impl Fn(&[ConversationBlock]) + Send + Sync + 'static,
    ) -> Self {
        self.on_change = Some(Box::new(callback));
        self
    }

    /// Register a callback invoked with the agent's refreshed list
    /// record after every applied AGENT-STATUS event (activation /
    /// deactivation / tag change — never for conversation events).
    /// Runs on the pump task — keep it cheap; for state on demand use
    /// [`agent`](WebSocketAgentsInstancesListener::agent).
    pub fn on_agent_change(
        mut self,
        callback: impl Fn(&AgentRecord) + Send + Sync + 'static,
    ) -> Self {
        self.on_agent_change = Some(Box::new(callback));
        self
    }

    /// Upgrade, send the auth preamble, and start the pump. The
    /// returned listener immediately begins folding the snapshot
    /// replay.
    pub async fn connect(self) -> Result<WebSocketAgentsInstancesListener, Error> {
        let upgrade = self
            .url
            .as_str()
            .into_client_request()
            .map_err(Error::Connect)?;
        let (mut ws, _response) = tokio_tungstenite::connect_async(upgrade)
            .await
            .map_err(Error::Connect)?;

        // The auth preamble — always the connection's first text frame,
        // `{"signature": null}` against a secretless daemon.
        let auth = serde_json::to_string(&AuthEnvelope {
            signature: self.signature,
        })
        .expect("AuthEnvelope serialization is infallible");
        ws.send(tungstenite::Message::Text(auth.into()))
            .await
            .map_err(Error::Ws)?;

        let (sink, stream) = ws.split();
        let shared = Arc::new(Shared {
            state: Mutex::new(ConversationState::default()),
            agent: Mutex::new(None),
            changes: watch::channel(0u64).0,
            on_change: self.on_change,
            on_agent_change: self.on_agent_change,
            sink: Mutex::new(sink),
        });
        let pump = tokio::spawn(pump(stream, shared.clone()));
        Ok(WebSocketAgentsInstancesListener { shared, pump })
    }
}

/// The materialized `/agents/instances/{*aih}` view — see the module
/// docs. Construct via [`WebSocketAgentsInstancesListener::new`].
/// Dropping it aborts the background pump.
pub struct WebSocketAgentsInstancesListener {
    shared: Arc<Shared>,
    pump: tokio::task::JoinHandle<()>,
}

impl WebSocketAgentsInstancesListener {
    /// Start building a listener for one agent's conversation URL (the
    /// daemon's published base address + `/agents/instances/` + the
    /// agent's hierarchy).
    pub fn new(url: impl Into<String>) -> WebSocketAgentsInstancesListenerBuilder {
        WebSocketAgentsInstancesListenerBuilder {
            url: url.into(),
            signature: None,
            on_change: None,
            on_agent_change: None,
        }
    }

    /// Snapshot the current conversation, blocks in conversation order.
    pub async fn conversation(&self) -> Vec<ConversationBlock> {
        self.shared.state.lock().await.conversation()
    }

    /// The agent's current list record (active flag, bound tags,
    /// counters) — the same shape `/agents/instances/list` tracks,
    /// scoped to this agent. `None` until the connection's first
    /// agent-status event lands (the daemon ships one right after
    /// auth). Structurally independent of
    /// [`conversation`](Self::conversation).
    pub async fn agent(&self) -> Option<AgentRecord> {
        self.shared.agent.lock().await.clone()
    }

    /// Whether the snapshot replay has completed — every event after
    /// the `Live` marker is the conversation as it occurs.
    pub async fn is_live(&self) -> bool {
        self.shared.state.lock().await.live
    }

    /// Block until the next event is applied. A fresh call waits for
    /// the FIRST event after it is made — pair with
    /// [`conversation`](Self::conversation) in a loop, or use the
    /// on-change callback for guaranteed push.
    pub async fn subscribe(&self) {
        let mut rx = self.shared.changes.subscribe();
        let _ = rx.changed().await;
    }
}

impl Drop for WebSocketAgentsInstancesListener {
    fn drop(&mut self) {
        // Stop updating a view no one holds any more.
        self.pump.abort();
    }
}

/// Read frames, fold each [`AgentInstanceEvent`] into its concern's
/// state — conversation events into the coalescer, agent-status events
/// into the record slot — fire that concern's callback (never the
/// other's), and bump the shared change counter. Runs until the
/// connection closes (the daemon disconnects lagging clients —
/// reconnect for a fresh snapshot). Unparseable frames are SKIPPED —
/// the forward-compat contract for future event variants.
async fn pump(mut stream: SplitStream<Ws>, shared: Arc<Shared>) {
    while let Some(message) = stream.next().await {
        let text = match message {
            Ok(tungstenite::Message::Text(text)) => text,
            // Control / non-text frames: tungstenite answers pings itself.
            Ok(tungstenite::Message::Close(_)) | Err(_) => break,
            Ok(_) => continue,
        };
        let Ok(event) = serde_json::from_str::<AgentInstanceEvent>(&text) else {
            continue;
        };
        match event {
            AgentInstanceEvent::Agent { agent } => {
                {
                    let mut slot = shared.agent.lock().await;
                    *slot = Some(agent.clone());
                }
                if let Some(callback) = &shared.on_agent_change {
                    callback(&agent);
                }
            }
            event => {
                let snapshot = {
                    let mut state = shared.state.lock().await;
                    state.apply(event);
                    shared.on_change.as_ref().map(|_| state.conversation())
                };
                if let (Some(callback), Some(snapshot)) = (&shared.on_change, &snapshot) {
                    callback(snapshot);
                }
            }
        }
        shared.changes.send_modify(|version| {
            *version = version.wrapping_add(1);
        });
    }
}
