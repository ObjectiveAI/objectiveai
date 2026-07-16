//! Materialized consumer of the cli daemon's `/agents/instances/{*aih}`
//! endpoint — one agent's full conversation, history + live.
//!
//! [`AgentsInstancesListener`] connects once, then folds every
//! incoming [`AgentInstanceEvent`] into an in-memory conversation: the
//! DB snapshot replays first, `Live` marks the seam, then live events
//! stream as the agent produces them. Every conversation event is a
//! keyed full-value upsert — a re-sent identity REPLACES the prior
//! part (later = more complete), which also converges any
//! snapshot/live overlap.
//!
//! The coalescer is the incremental analogue of the CLI's `read_all`
//! block builder, and materializes the SAME shape: consecutive events
//! sharing their block's boundary fields (each event variant carries
//! exactly its class's boundary — response_id, plus `tool_call_id` /
//! sender + queue id where the class has one) join the LAST block;
//! anything else opens a new one. A re-sent identity never re-runs the
//! boundary test — it routes straight back to its part. Single-row
//! blocks (`vote` / `error`) arrive complete. Blocks materialize as
//! [`ConversationBlock`]s — the `agents logs list` `ResponseItem`
//! mirror — in conversation order.
//!
//! Three ways to observe, mirroring
//! [`super::super::agents_instances_list_listener`]:
//! [`conversation`](AgentsInstancesListener::conversation),
//! an on-change callback, and
//! [`subscribe`](AgentsInstancesListener::subscribe).
//!
//! One listener = one connection: the view updates until the SSE
//! stream closes (the daemon disconnects lagging clients rather than
//! dropping frames), then freezes. Reconnection is the caller's loop —
//! build a new listener; the fresh snapshot replaces everything. This
//! is a read-only stream; a future client→daemon message channel would
//! be a separate request, not this one.

use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;

use futures::StreamExt;
use reqwest_eventsource::{Event, RequestBuilderExt};
use tokio::sync::{Mutex, watch};

use super::{
    AgentInstanceEvent, AgentRecord, AssistantResponsePart, ClientNotificationPart,
    ConversationBlock, RequestMessageUserPart, ToolResponsePart, VectorRequestChoice,
    VectorRequestChoicePart,
};

/// The conversation on-change callback: invoked with the full current
/// conversation (blocks in conversation order) after each applied
/// conversation event.
pub type OnChange = Box<dyn Fn(&[ConversationBlock]) + Send + Sync>;

/// The agent-status on-change callback: invoked with the agent's
/// refreshed list record after each applied `Agent` event. Structurally
/// independent of the conversation callback — neither fires for the
/// other's events.
pub type OnAgentChange = Box<dyn Fn(&AgentRecord) + Send + Sync>;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// The request builder rejected the URL, or opening the SSE
    /// stream failed.
    #[error("connect daemon sse: {0}")]
    Connect(#[from] reqwest_eventsource::CannotCloneRequestError),
    /// The underlying reqwest client failed to build.
    #[error("daemon sse http client: {0}")]
    Client(#[from] reqwest::Error),
}

/// The class discriminant for part identity routing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum Class {
    RequestMessageUser,
    RequestMessageAssistant,
    RequestMessageTool,
    VectorRequestChoices,
    VectorResponseVote,
    ClientNotification,
    AssistantResponse,
    ToolResponse,
}

/// A part's replace-at key: identical between the snapshot replay and
/// the live tee. AIH is constant per connection and omitted;
/// `response_id` is `None` only on pre-stream `error` events.
type PartIdentity = (Class, Option<String>, i64, Option<i64>);

/// One multi-part block's boundary — `read_all`'s exact tuple, typed
/// per class. Two consecutive events join one block iff their keys are
/// EQUAL. Single-row classes (vote / error) have no key: they never
/// join anything.
#[derive(Debug, Clone, PartialEq)]
enum BlockKey {
    RequestMessageUser {
        response_id: String,
    },
    RequestMessageAssistant {
        response_id: String,
    },
    RequestMessageTool {
        response_id: String,
        tool_call_id: String,
    },
    VectorRequestChoices {
        response_id: String,
    },
    ClientNotification {
        response_id: String,
        sender_agent_instance_hierarchy: String,
        message_queue_id: i64,
    },
    AssistantResponse {
        response_id: String,
    },
    ToolResponse {
        response_id: String,
        tool_call_id: String,
    },
}

/// One in-progress multi-part block: its boundary key plus its typed
/// parts, ordered by the DB row identity `(row_index, row_sub_index)`
/// — arrival-independent, so a late replacement lands in place.
struct OpenBlock {
    agent_instance_hierarchy: String,
    key: BlockKey,
    body: OpenBody,
}

enum OpenBody {
    /// `RequestMessageUser` parts.
    User(BTreeMap<(i64, Option<i64>), RequestMessageUserPart>),
    /// `AssistantResponse` / `RequestMessageAssistant` parts (the key
    /// distinguishes the two classes).
    Assistant(BTreeMap<(i64, Option<i64>), AssistantResponsePart>),
    /// `ToolResponse` / `RequestMessageTool` parts.
    Tool(BTreeMap<(i64, Option<i64>), ToolResponsePart>),
    /// `ClientNotification` parts + the block-level queue metadata
    /// (refreshed per event; constant per parent queue row).
    Notification {
        queued_at: String,
        key: Option<String>,
        parts: BTreeMap<(i64, Option<i64>), ClientNotificationPart>,
    },
    /// `VectorRequestChoices`: per `(choice_index, part_index)`, the
    /// choice's voting key (refreshed per event) + the part.
    Choices(BTreeMap<(i64, i64), (String, VectorRequestChoicePart)>),
}

/// One conversation slot: an in-progress multi-part block, or a
/// complete single-row block (vote / error — replaced whole on
/// identity re-send).
enum Slot {
    Open(OpenBlock),
    Single(ConversationBlock),
}

impl OpenBlock {
    /// Materialize — the `ResponseItem` mirror shape.
    fn to_block(&self) -> ConversationBlock {
        let aih = self.agent_instance_hierarchy.clone();
        match (&self.key, &self.body) {
            (BlockKey::RequestMessageUser { response_id }, OpenBody::User(parts)) => {
                ConversationBlock::RequestMessageUser {
                    agent_instance_hierarchy: aih,
                    response_id: response_id.clone(),
                    parts: parts.values().cloned().collect(),
                }
            }
            (BlockKey::RequestMessageAssistant { response_id }, OpenBody::Assistant(parts)) => {
                ConversationBlock::RequestMessageAssistant {
                    agent_instance_hierarchy: aih,
                    response_id: response_id.clone(),
                    parts: parts.values().cloned().collect(),
                }
            }
            (
                BlockKey::RequestMessageTool {
                    response_id,
                    tool_call_id,
                },
                OpenBody::Tool(parts),
            ) => ConversationBlock::RequestMessageTool {
                agent_instance_hierarchy: aih,
                response_id: response_id.clone(),
                tool_call_id: tool_call_id.clone(),
                parts: parts.values().cloned().collect(),
            },
            (BlockKey::VectorRequestChoices { response_id }, OpenBody::Choices(parts)) => {
                // Ordered by (choice_index, part_index); group runs of
                // one choice index.
                let mut choices: Vec<VectorRequestChoice> = Vec::new();
                let mut current: Option<(i64, VectorRequestChoice)> = None;
                for ((choice_index, _part_index), (key, part)) in parts {
                    match &mut current {
                        Some((index, choice)) if index == choice_index => {
                            choice.key = key.clone();
                            choice.parts.push(part.clone());
                        }
                        _ => {
                            if let Some((_, done)) = current.take() {
                                choices.push(done);
                            }
                            current = Some((
                                *choice_index,
                                VectorRequestChoice {
                                    key: key.clone(),
                                    parts: vec![part.clone()],
                                },
                            ));
                        }
                    }
                }
                if let Some((_, done)) = current.take() {
                    choices.push(done);
                }
                ConversationBlock::VectorRequestChoices {
                    agent_instance_hierarchy: aih,
                    response_id: response_id.clone(),
                    choices,
                }
            }
            (
                BlockKey::ClientNotification {
                    response_id,
                    sender_agent_instance_hierarchy,
                    message_queue_id: _,
                },
                OpenBody::Notification {
                    queued_at,
                    key,
                    parts,
                },
            ) => ConversationBlock::ClientNotification {
                agent_instance_hierarchy: aih,
                sender_agent_instance_hierarchy: sender_agent_instance_hierarchy.clone(),
                response_id: response_id.clone(),
                queued_at: queued_at.clone(),
                key: key.clone(),
                parts: parts.values().cloned().collect(),
            },
            (BlockKey::AssistantResponse { response_id }, OpenBody::Assistant(parts)) => {
                ConversationBlock::AssistantResponse {
                    agent_instance_hierarchy: aih,
                    response_id: response_id.clone(),
                    parts: parts.values().cloned().collect(),
                }
            }
            (
                BlockKey::ToolResponse {
                    response_id,
                    tool_call_id,
                },
                OpenBody::Tool(parts),
            ) => ConversationBlock::ToolResponse {
                agent_instance_hierarchy: aih,
                response_id: response_id.clone(),
                tool_call_id: tool_call_id.clone(),
                parts: parts.values().cloned().collect(),
            },
            // Key/body pairing is fixed at construction — see
            // `ConversationState::apply_part`.
            _ => unreachable!("BlockKey/OpenBody pairing is fixed at construction"),
        }
    }
}

/// The materialized conversation, folded incrementally.
#[derive(Default)]
struct ConversationState {
    /// Arrival order == conversation order (snapshot `"index"` order,
    /// then live write order).
    slots: Vec<Slot>,
    /// Replace-at-key routing: a re-sent part goes straight back to
    /// its slot, never through the boundary test.
    part_to_slot: HashMap<PartIdentity, usize>,
    live: bool,
}

/// Everything the coalescer needs from one conversation part event:
/// its identity, its block's boundary, and the typed part.
struct PartEvent {
    identity: PartIdentity,
    agent_instance_hierarchy: String,
    key: BlockKey,
    part: PartPayload,
}

enum PartPayload {
    User(RequestMessageUserPart),
    Assistant(AssistantResponsePart),
    Tool(ToolResponsePart),
    Notification {
        queued_at: String,
        key: Option<String>,
        part: ClientNotificationPart,
    },
    Choice {
        key: String,
        part: VectorRequestChoicePart,
    },
}

impl ConversationState {
    fn apply(&mut self, event: AgentInstanceEvent) {
        match event {
            AgentInstanceEvent::Live => self.live = true,
            // Agent-status events never reach the conversation state —
            // the pump routes them to their own slot.
            AgentInstanceEvent::Agent { .. } => {}
            AgentInstanceEvent::RequestMessageUserPart {
                agent_instance_hierarchy,
                response_id,
                row_index,
                row_sub_index,
                part,
            } => self.apply_part(PartEvent {
                identity: (
                    Class::RequestMessageUser,
                    Some(response_id.clone()),
                    row_index,
                    row_sub_index,
                ),
                agent_instance_hierarchy,
                key: BlockKey::RequestMessageUser { response_id },
                part: PartPayload::User(part),
            }),
            AgentInstanceEvent::RequestMessageAssistantPart {
                agent_instance_hierarchy,
                response_id,
                row_index,
                row_sub_index,
                part,
            } => self.apply_part(PartEvent {
                identity: (
                    Class::RequestMessageAssistant,
                    Some(response_id.clone()),
                    row_index,
                    row_sub_index,
                ),
                agent_instance_hierarchy,
                key: BlockKey::RequestMessageAssistant { response_id },
                part: PartPayload::Assistant(part),
            }),
            AgentInstanceEvent::RequestMessageToolPart {
                agent_instance_hierarchy,
                response_id,
                tool_call_id,
                row_index,
                row_sub_index,
                part,
            } => self.apply_part(PartEvent {
                identity: (
                    Class::RequestMessageTool,
                    Some(response_id.clone()),
                    row_index,
                    row_sub_index,
                ),
                agent_instance_hierarchy,
                key: BlockKey::RequestMessageTool {
                    response_id,
                    tool_call_id,
                },
                part: PartPayload::Tool(part),
            }),
            AgentInstanceEvent::VectorRequestChoicePart {
                agent_instance_hierarchy,
                response_id,
                key,
                choice_index,
                part_index,
                part,
            } => self.apply_part(PartEvent {
                identity: (
                    Class::VectorRequestChoices,
                    Some(response_id.clone()),
                    choice_index,
                    Some(part_index),
                ),
                agent_instance_hierarchy,
                key: BlockKey::VectorRequestChoices { response_id },
                part: PartPayload::Choice { key, part },
            }),
            AgentInstanceEvent::ClientNotificationPart {
                agent_instance_hierarchy,
                response_id,
                sender_agent_instance_hierarchy,
                message_queue_id,
                queued_at,
                key,
                row_index,
                part,
            } => self.apply_part(PartEvent {
                identity: (
                    Class::ClientNotification,
                    Some(response_id.clone()),
                    row_index,
                    None,
                ),
                agent_instance_hierarchy,
                key: BlockKey::ClientNotification {
                    response_id,
                    sender_agent_instance_hierarchy,
                    message_queue_id,
                },
                part: PartPayload::Notification {
                    queued_at,
                    key,
                    part,
                },
            }),
            AgentInstanceEvent::AssistantResponsePart {
                agent_instance_hierarchy,
                response_id,
                row_index,
                row_sub_index,
                part,
            } => self.apply_part(PartEvent {
                identity: (
                    Class::AssistantResponse,
                    Some(response_id.clone()),
                    row_index,
                    row_sub_index,
                ),
                agent_instance_hierarchy,
                key: BlockKey::AssistantResponse { response_id },
                part: PartPayload::Assistant(part),
            }),
            AgentInstanceEvent::ToolResponsePart {
                agent_instance_hierarchy,
                response_id,
                tool_call_id,
                row_index,
                row_sub_index,
                part,
            } => self.apply_part(PartEvent {
                identity: (
                    Class::ToolResponse,
                    Some(response_id.clone()),
                    row_index,
                    row_sub_index,
                ),
                agent_instance_hierarchy,
                key: BlockKey::ToolResponse {
                    response_id,
                    tool_call_id,
                },
                part: PartPayload::Tool(part),
            }),
            AgentInstanceEvent::VectorResponseVote {
                agent_instance_hierarchy,
                response_id,
                vote,
            } => self.apply_single(
                (
                    Class::VectorResponseVote,
                    Some(response_id.clone()),
                    0,
                    None,
                ),
                ConversationBlock::VectorResponseVote {
                    agent_instance_hierarchy,
                    response_id,
                    vote,
                },
            ),
            AgentInstanceEvent::Error {
                agent_instance_hierarchy,
                response_id,
                error,
                delivered_at,
            } => self.apply_error(ConversationBlock::Error {
                agent_instance_hierarchy,
                response_id,
                error,
                delivered_at,
            }),
        }
    }

    /// Fold one part in: a known identity routes straight back to its
    /// slot; a new identity joins the LAST slot iff it is an open
    /// block with an EQUAL boundary key, else opens a new block.
    fn apply_part(&mut self, event: PartEvent) {
        let slot = if let Some(&index) = self.part_to_slot.get(&event.identity) {
            index
        } else {
            let joins_last = matches!(
                self.slots.last(),
                Some(Slot::Open(open)) if open.key == event.key
            );
            let index = if joins_last {
                self.slots.len() - 1
            } else {
                self.slots.push(Slot::Open(OpenBlock {
                    agent_instance_hierarchy: event.agent_instance_hierarchy.clone(),
                    key: event.key.clone(),
                    body: match &event.part {
                        PartPayload::User(_) => OpenBody::User(BTreeMap::new()),
                        PartPayload::Assistant(_) => OpenBody::Assistant(BTreeMap::new()),
                        PartPayload::Tool(_) => OpenBody::Tool(BTreeMap::new()),
                        PartPayload::Notification { .. } => OpenBody::Notification {
                            queued_at: String::new(),
                            key: None,
                            parts: BTreeMap::new(),
                        },
                        PartPayload::Choice { .. } => OpenBody::Choices(BTreeMap::new()),
                    },
                }));
                self.slots.len() - 1
            };
            self.part_to_slot.insert(event.identity.clone(), index);
            index
        };
        let Some(Slot::Open(open)) = self.slots.get_mut(slot) else {
            return; // Identity collision with a single-row slot — impossible by class.
        };
        let (_, _, row_index, row_sub_index) = event.identity;
        match (event.part, &mut open.body) {
            (PartPayload::User(part), OpenBody::User(parts)) => {
                parts.insert((row_index, row_sub_index), part);
            }
            (PartPayload::Assistant(part), OpenBody::Assistant(parts)) => {
                parts.insert((row_index, row_sub_index), part);
            }
            (PartPayload::Tool(part), OpenBody::Tool(parts)) => {
                parts.insert((row_index, row_sub_index), part);
            }
            (
                PartPayload::Notification {
                    queued_at,
                    key,
                    part,
                },
                OpenBody::Notification {
                    queued_at: block_queued_at,
                    key: block_key,
                    parts,
                },
            ) => {
                *block_queued_at = queued_at;
                *block_key = key;
                parts.insert((row_index, row_sub_index), part);
            }
            (PartPayload::Choice { key, part }, OpenBody::Choices(parts)) => {
                parts.insert(
                    (row_index, row_sub_index.unwrap_or(0)),
                    (key, part),
                );
            }
            // Pairing fixed at construction (the body was built FROM
            // this payload's class); a same-identity payload of a
            // different class cannot exist.
            _ => {}
        }
    }

    /// Fold one error block in. Errors are IMMUTABLE and carry no
    /// replace-at identity — the snapshot/live seam (the one way the
    /// same error arrives twice) dedupes by VALUE equality.
    fn apply_error(&mut self, block: ConversationBlock) {
        let duplicate = self
            .slots
            .iter()
            .any(|slot| matches!(slot, Slot::Single(existing) if *existing == block));
        if !duplicate {
            self.slots.push(Slot::Single(block));
        }
    }

    /// Fold one complete single-row block in: replace at a known
    /// identity, else append.
    fn apply_single(&mut self, identity: PartIdentity, block: ConversationBlock) {
        if let Some(&index) = self.part_to_slot.get(&identity) {
            if let Some(slot) = self.slots.get_mut(index) {
                *slot = Slot::Single(block);
            }
            return;
        }
        self.slots.push(Slot::Single(block));
        self.part_to_slot.insert(identity, self.slots.len() - 1);
    }

    fn conversation(&self) -> Vec<ConversationBlock> {
        self.slots
            .iter()
            .map(|slot| match slot {
                Slot::Open(open) => open.to_block(),
                Slot::Single(block) => block.clone(),
            })
            .collect()
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
    /// [`subscribe`](AgentsInstancesListener::subscribe) waiter.
    changes: watch::Sender<u64>,
    on_change: Option<OnChange>,
    on_agent_change: Option<OnAgentChange>,
}

/// Unconnected configuration — [`AgentsInstancesListener::new`] +
/// builder methods + [`AgentsInstancesListenerBuilder::connect`].
pub struct AgentsInstancesListenerBuilder {
    /// Full connect URL: the daemon's published base address +
    /// `/agents/instances/` + the agent's hierarchy, e.g.
    /// `http://127.0.0.1:49152/agents/instances/root/child-abc`.
    url: String,
    signature: Option<String>,
    on_change: Option<OnChange>,
    on_agent_change: Option<OnAgentChange>,
}

impl AgentsInstancesListenerBuilder {
    /// Attach the daemon auth signature (the pre-derived
    /// `sha256=<hex(SHA256(DAEMON_SECRET))>`), sent as the
    /// `X-OBJECTIVEAI-SIGNATURE` request header — the daemon's SSE
    /// watcher routes authenticate by header. Without it the daemon
    /// must be running without a secret.
    pub fn signature(mut self, signature: impl Into<String>) -> Self {
        self.signature = Some(signature.into());
        self
    }

    /// Register a callback invoked with the full current conversation
    /// after every applied CONVERSATION event (never for agent-status
    /// events). Runs on the pump task — keep it cheap; for state on
    /// demand use
    /// [`conversation`](AgentsInstancesListener::conversation).
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
    /// [`agent`](AgentsInstancesListener::agent).
    pub fn on_agent_change(
        mut self,
        callback: impl Fn(&AgentRecord) + Send + Sync + 'static,
    ) -> Self {
        self.on_agent_change = Some(Box::new(callback));
        self
    }

    /// Open the SSE stream and start the pump. The returned listener
    /// immediately begins folding the snapshot replay.
    pub async fn connect(self) -> Result<AgentsInstancesListener, Error> {
        let source = connect_sse(&self.url, self.signature.as_deref())?;
        let shared = Arc::new(Shared {
            state: Mutex::new(ConversationState::default()),
            agent: Mutex::new(None),
            changes: watch::channel(0u64).0,
            on_change: self.on_change,
            on_agent_change: self.on_agent_change,
        });
        let pump = tokio::spawn(pump(source, shared.clone()));
        Ok(AgentsInstancesListener { shared, pump })
    }
}

/// Open the daemon's SSE watcher stream: request
/// `text/event-stream`, and stamp `X-OBJECTIVEAI-SIGNATURE` when a
/// signature is present (the daemon's watcher routes moved auth from
/// the first-frame preamble to this header).
fn connect_sse(
    url: &str,
    signature: Option<&str>,
) -> Result<reqwest_eventsource::EventSource, Error> {
    let client = reqwest::Client::builder().build()?;
    let mut request = client
        .get(url)
        .header("Accept", "text/event-stream");
    if let Some(signature) = signature {
        request = request.header("X-OBJECTIVEAI-SIGNATURE", signature);
    }
    Ok(request.eventsource()?)
}

/// The materialized `/agents/instances/{*aih}` view — see the module
/// docs. Construct via [`AgentsInstancesListener::new`].
/// Dropping it aborts the background pump.
pub struct AgentsInstancesListener {
    shared: Arc<Shared>,
    pump: tokio::task::JoinHandle<()>,
}

impl AgentsInstancesListener {
    /// Start building a listener for one agent's conversation URL (the
    /// daemon's published base address + `/agents/instances/` + the
    /// agent's hierarchy).
    pub fn new(url: impl Into<String>) -> AgentsInstancesListenerBuilder {
        AgentsInstancesListenerBuilder {
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

impl Drop for AgentsInstancesListener {
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
async fn pump(mut source: reqwest_eventsource::EventSource, shared: Arc<Shared>) {
    while let Some(event_result) = source.next().await {
        let message = match event_result {
            Ok(Event::Open) => continue,
            Ok(Event::Message(message)) => message,
            Err(_) => break,
        };
        let Ok(event) = serde_json::from_str::<AgentInstanceEvent>(&message.data) else {
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
