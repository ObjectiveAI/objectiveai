//! The desk: where the run and the person meet.
//!
//! One per container, shared by the run and every open page. It holds
//! the transcript, the open turn, if any, and the fan-out that tells
//! every page what happens.
//!
//! The rules:
//!
//! 1. A turn opens when a run starts, and at most one is open. A run
//!    that comes while one is open is refused, unless the open one's
//!    run is gone — the proxy stopped reading it — which closes it.
//! 2. Every chunk is recorded the same way: given the next place in
//!    the transcript, kept in the database, sent on the open turn's
//!    run, and told to every page. A chunk recorded with no turn open
//!    is kept and told, and goes on no run.
//! 3. The person's acts — a reply, a tool call, the turn's end — need
//!    an open turn; with none, each is refused, with why.
//! 4. A reply is one `assistant_text_content` chunk.
//! 5. A tool call is an `assistant_tool_call` chunk under an id minted
//!    here, its arguments as JSON text; the tool is called through the
//!    proxy's MCP, outside the lock, and its result recorded as the
//!    `tool_response` under the same id. A call that fails as a link
//!    is recorded as an error result, so every call has its response.
//! 6. The turn's end ends the run: the turn's sender is dropped, which
//!    ends the stream. It is refused while a tool call is in flight,
//!    so no response is left off the run.
//! 7. A message enqueued while a turn is open — the proxy's joined
//!    batch — is taken at once: every part is recorded under its own
//!    message's key, and it is `delivered`. With none open it is
//!    `missed`, and the proxy starts the next run on it.
//! 8. The transcript is read from the database the first time the desk
//!    is used, and kept in memory after.

use std::sync::Arc;

use diverge_sdk::container_proxy::inside::Client;
use diverge_sdk::container_proxy::inside::agent::enqueue::Fate;
use diverge_sdk::provider::endpoints::containers::agents::run::server::response::{
    AgenticLoopChunk, AssistantTextContentChunk, AssistantToolCallChunk, ToolResponseChunk, user_parts,
};
use rmcp::model::{CallToolRequestParams, CallToolResult, ContentBlock, JsonObject, Tool};
use serde::Serialize;
use tokio::sync::{Mutex, MutexGuard, broadcast, mpsc};

use crate::store::{Entry, Store};

/// What a page is told.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Told {
    /// A chunk recorded.
    Entry { entry: Entry },
    /// Whether a turn is open: the agent waiting on the person.
    State { waiting: bool },
    /// Something went wrong that is no one act's: a chunk that could
    /// not be kept.
    Trouble { error: String },
}

/// The desk, shared.
#[derive(Clone)]
pub struct Desk(Arc<Shared>);

struct Shared {
    inner: Mutex<Inner>,
    told: broadcast::Sender<Told>,
    client: Client,
    store: Store,
}

#[derive(Default)]
struct Inner {
    /// Whether the transcript has been read (rule 8).
    loaded: bool,
    transcript: Vec<Entry>,
    /// The open turn's run.
    turn: Option<mpsc::UnboundedSender<AgenticLoopChunk>>,
    /// The ids minted so far, for the next.
    calls: u64,
    /// The tool calls in flight.
    pending: usize,
}

/// Why a run was not started.
pub enum Busy {
    /// A turn is open (rule 1).
    Open,
}

impl Desk {
    /// A desk with nothing on it.
    pub fn new() -> Self {
        let (told, _) = broadcast::channel(256);
        Desk(Arc::new(Shared { inner: Mutex::new(Inner::default()), told, client: Client::new(), store: Store::default() }))
    }

    /// Listen for what pages are told.
    pub fn listen(&self) -> broadcast::Receiver<Told> {
        self.0.told.subscribe()
    }

    /// The lock, the transcript read first if it has not been (rule 8).
    async fn lock(&self) -> MutexGuard<'_, Inner> {
        let mut inner = self.0.inner.lock().await;
        if !inner.loaded {
            match self.0.store.load(&self.0.client).await {
                Ok(transcript) => {
                    inner.transcript = transcript;
                    inner.loaded = true;
                }
                Err(error) => {
                    let _ = self.0.told.send(Told::Trouble { error: format!("the transcript could not be read: {error}") });
                }
            }
        }
        inner
    }

    /// Whether a turn is open, closing one whose run is gone (rule 1).
    fn open(&self, inner: &mut Inner) -> bool {
        if inner.turn.as_ref().is_some_and(|turn| turn.is_closed()) {
            inner.turn = None;
            inner.pending = 0;
            let _ = self.0.told.send(Told::State { waiting: false });
        }
        inner.turn.is_some()
    }

    /// Rule 2.
    async fn record(&self, inner: &mut Inner, chunk: AgenticLoopChunk) {
        let entry = Entry { id: inner.transcript.last().map_or(0, |last| last.id + 1), chunk };
        if let Err(error) = self.0.store.append(&self.0.client, &entry).await {
            let _ = self.0.told.send(Told::Trouble { error: format!("a chunk could not be kept: {error}") });
        }
        if let Some(turn) = &inner.turn {
            let _ = turn.send(entry.chunk.clone());
        }
        inner.transcript.push(entry.clone());
        let _ = self.0.told.send(Told::Entry { entry });
    }

    /// The transcript and whether a turn is open, as a page first sees
    /// them.
    pub async fn snapshot(&self) -> (Vec<Entry>, bool) {
        let mut inner = self.lock().await;
        let waiting = self.open(&mut inner);
        (inner.transcript.clone(), waiting)
    }

    /// The tools the agent may call, through the proxy's listing.
    pub async fn tools(&self) -> Result<Vec<Tool>, String> {
        let peer = self.0.client.mcp_peer().await.map_err(|error| error.to_string())?;
        peer.list_all_tools().await.map_err(|error| error.to_string())
    }

    /// Rule 1: a turn opened on the run's messages, their parts
    /// recorded first; the chunks of the run, until the turn ends.
    pub async fn begin(&self, messages: Vec<(String, Vec<ContentBlock>)>) -> Result<mpsc::UnboundedReceiver<AgenticLoopChunk>, Busy> {
        let mut inner = self.lock().await;
        if self.open(&mut inner) {
            return Err(Busy::Open);
        }
        let (turn, run) = mpsc::unbounded_channel();
        inner.turn = Some(turn);
        inner.pending = 0;
        for (key, content) in messages {
            for chunk in user_parts(&key, content) {
                self.record(&mut inner, chunk).await;
            }
        }
        let _ = self.0.told.send(Told::State { waiting: true });
        Ok(run)
    }

    /// Rule 7.
    pub async fn deliver(&self, messages: Vec<(String, Vec<ContentBlock>)>) -> Fate {
        let mut inner = self.lock().await;
        if !self.open(&mut inner) {
            return Fate::Missed;
        }
        for (key, content) in messages {
            for chunk in user_parts(&key, content) {
                self.record(&mut inner, chunk).await;
            }
        }
        Fate::Delivered
    }

    /// Rule 4.
    pub async fn reply(&self, text: String) -> Result<(), String> {
        let mut inner = self.lock().await;
        if !self.open(&mut inner) {
            return Err("no message is waiting on you".to_string());
        }
        let chunk = AssistantTextContentChunk { r#type: Default::default(), parent_tool_call_id: None, logprobs: None, inner: rmcp::model::TextContent::new(text) };
        self.record(&mut inner, AgenticLoopChunk::AssistantTextContent(chunk)).await;
        Ok(())
    }

    /// Rule 5, its first half: the call recorded, or refused. The
    /// [`Placed`] makes it.
    pub async fn place(&self, name: String, arguments: JsonObject) -> Result<Placed, String> {
        let mut inner = self.lock().await;
        if !self.open(&mut inner) {
            return Err("no message is waiting on you".to_string());
        }
        let text = serde_json::to_string(&arguments).map_err(|error| error.to_string())?;
        inner.calls += 1;
        inner.pending += 1;
        let id = format!("call_{}", inner.calls);
        let chunk = AssistantToolCallChunk { r#type: Default::default(), parent_tool_call_id: None, id: id.clone(), meta: None, name: name.clone(), arguments: Some(text) };
        self.record(&mut inner, AgenticLoopChunk::AssistantToolCall(chunk)).await;
        Ok(Placed { desk: self.clone(), id, name, arguments })
    }

    /// Rule 6.
    pub async fn end(&self) -> Result<(), String> {
        let mut inner = self.lock().await;
        if !self.open(&mut inner) {
            return Err("no message is waiting on you".to_string());
        }
        if inner.pending > 0 {
            return Err("a tool call is still running".to_string());
        }
        inner.turn = None;
        let _ = self.0.told.send(Told::State { waiting: false });
        Ok(())
    }
}

/// A tool call recorded and not yet made.
pub struct Placed {
    desk: Desk,
    id: String,
    name: String,
    arguments: JsonObject,
}

impl Placed {
    /// Rule 5, its second half: the tool called through the proxy's
    /// MCP, outside the lock, and its result recorded under the call's
    /// id; a call that fails as a link, as an error result.
    pub async fn finish(self) {
        let Placed { desk, id, name, arguments } = self;
        let mut params = CallToolRequestParams::new(name);
        params.arguments = Some(arguments);
        let result = match desk.0.client.mcp_call_tool(params).await {
            Ok(result) => result,
            Err(error) => CallToolResult::error(vec![ContentBlock::text(format!("the tool could not be called: {error}"))]),
        };
        let mut inner = desk.lock().await;
        inner.pending = inner.pending.saturating_sub(1);
        let chunk = ToolResponseChunk { r#type: Default::default(), parent_tool_call_id: None, id, inner: result };
        desk.record(&mut inner, AgenticLoopChunk::ToolResponse(chunk)).await;
    }
}
