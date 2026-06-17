//! API-side implementation of `objectiveai_mcp_proxy::QueueDelegate`.
//!
//! The MCP proxy embedded in the API process is configured (at
//! [`super::proxy::ProxySpawner`] boot) with a singleton
//! [`ApiQueueDelegate`]. The delegate routes calls by AIH to
//! per-loop state — `run_agent_loop` registers an AIH at startup
//! with its reverse-attach handle and an initial confirmed set,
//! and unregisters at end of stream.
//!
//! State machine per AIH:
//!
//! - **`confirmed`** — content_ids known to have actually reached
//!   the agent. Seeded at `register()` time by the run-loop's
//!   own startup queue snapshot, then grown by `confirm()` calls
//!   triggered when the run-loop regex-scans a tool message and
//!   spots the matching prefix token. Reads filter against this
//!   set; rows here never re-issue.
//! - **`pending`** — speculatively-issued content_ids keyed by the
//!   confirmation token the delegate generated at read time.
//!   Reads also filter against the union of every pending list so
//!   a half-delivered batch can't re-issue mid-flight. Tokens
//!   the run-loop never sees stay in pending until `unregister()`
//!   drops the whole map; those ids then re-deliver on the next
//!   loop (which is the robustness win — we never claim delivery
//!   for content the agent didn't echo back).
//! - **`undelivered`** — ids the run-loop hasn't stamped onto a
//!   `ToolResponse.request_message_ids` yet. Drained on each
//!   confirm OR via `drain_undelivered()` as a defensive
//!   fallback.

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use std::future::Future;
use std::pin::Pin;

use indexmap::IndexMap;
use objectiveai_mcp_proxy::{QueueDelegate, QueueRead};
use objectiveai_sdk::client_objectiveai_mcp::server_response::ReadMessageQueueResult;
use objectiveai_sdk::mcp::tool::ContentBlock;
use tokio::sync::Mutex;

use crate::objectiveai_mcp::ReverseAttachHandle;

/// HTTP header carrying the agent instance hierarchy. The proxy
/// stamps this on every per-session transient header map; the
/// delegate reads it to look up per-loop state.
const AIH_HEADER: &str = "X-OBJECTIVEAI-AGENT-INSTANCE-HIERARCHY";

/// WS reverse-attach round-trip cap — same value
/// `read_message_queue_via_ws` uses for the startup snapshot.
const READ_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(30);

/// Singleton owned by [`super::proxy::ProxySpawner`] and passed
/// into `objectiveai_mcp_proxy::setup()` as the queue delegate.
pub struct ApiQueueDelegate {
    states: Mutex<HashMap<String, Arc<PerLoopState>>>,
}

impl std::fmt::Debug for ApiQueueDelegate {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ApiQueueDelegate").finish_non_exhaustive()
    }
}

struct PerLoopState {
    /// Reverse-attach back to the CLI for this loop. Captured at
    /// `register()` time; same handle the run-loop uses for its
    /// own queue reads + WS RPC.
    reverse_attach: Arc<ReverseAttachHandle>,
    inner: Mutex<PerLoopInner>,
}

struct PerLoopInner {
    confirmed: HashSet<i64>,
    pending: HashMap<String, Vec<i64>>,
    undelivered: Vec<i64>,
}

impl ApiQueueDelegate {
    pub fn new() -> Self {
        Self { states: Mutex::new(HashMap::new()) }
    }

    /// Called by `run_agent_loop` at startup. `initial_banned`
    /// goes straight into `confirmed` (the loop's startup
    /// snapshot already stamped them on the first assistant
    /// chunk — they really are delivered). Replaces any prior
    /// state for this AIH; a fresh registration starts with a
    /// clean pending/undelivered slate.
    pub async fn register(
        &self,
        aih: String,
        reverse_attach: Arc<ReverseAttachHandle>,
        initial_banned: Vec<i64>,
    ) {
        let mut confirmed: HashSet<i64> = HashSet::with_capacity(initial_banned.len());
        confirmed.extend(initial_banned);
        let state = PerLoopState {
            reverse_attach,
            inner: Mutex::new(PerLoopInner {
                confirmed,
                pending: HashMap::new(),
                undelivered: Vec::new(),
            }),
        };
        self.states.lock().await.insert(aih, Arc::new(state));
    }

    /// Promote `token`'s ids from `pending` to `confirmed` and
    /// append them to `undelivered`. Idempotent — unknown token
    /// (already consumed, never issued, mistyped) returns an
    /// empty Vec with no side effect.
    pub async fn confirm(&self, aih: &str, token: &str) -> Vec<i64> {
        let Some(state) = self.states.lock().await.get(aih).cloned() else {
            return Vec::new();
        };
        let mut inner = state.inner.lock().await;
        let Some(ids) = inner.pending.remove(token) else {
            return Vec::new();
        };
        inner.confirmed.extend(ids.iter().copied());
        inner.undelivered.extend(ids.iter().copied());
        ids
    }

    /// Drain any ids the run-loop hasn't stamped yet. Defensive
    /// — `confirm()` already returns the ids it just confirmed,
    /// so the typical call path stamps from that return value
    /// and this method is mostly for cleanup races.
    pub async fn drain_undelivered(&self, aih: &str) -> Vec<i64> {
        let Some(state) = self.states.lock().await.get(aih).cloned() else {
            return Vec::new();
        };
        let mut inner = state.inner.lock().await;
        std::mem::take(&mut inner.undelivered)
    }

    /// Drop per-AIH state. Pending tokens that never got confirmed
    /// vanish along with the loop; their ids re-deliver on the
    /// next loop's reads (the queue rows are still
    /// `active = TRUE` until the LogWriter flips them, which only
    /// happens when an actual `MessageQueueContent` log row gets
    /// written — i.e. when the agent really did consume them).
    pub async fn unregister(&self, aih: &str) {
        self.states.lock().await.remove(aih);
    }
}

impl Default for ApiQueueDelegate {
    fn default() -> Self {
        Self::new()
    }
}

impl QueueDelegate for ApiQueueDelegate {
    fn read_pending_blocks<'a>(
        &'a self,
        agent_arguments: &'a IndexMap<String, String>,
        _mcp_session_id: &'a str,
    ) -> Pin<Box<dyn Future<Output = Option<QueueRead>> + Send + 'a>> {
        Box::pin(async move {
            // Locate the per-loop state by AIH header. Lookup is
            // case-insensitive — clients normalize but headers
            // can arrive in either form.
            let aih = agent_arguments
                .iter()
                .find(|(k, _)| k.eq_ignore_ascii_case(AIH_HEADER))
                .map(|(_, v)| v.as_str())?;
            let state = self.states.lock().await.get(aih).cloned()?;

            // Snapshot the filter set: every id we've ever handed
            // out this loop, confirmed or pending. Held briefly
            // under one lock so concurrent reads see a consistent
            // view.
            let (filter, reverse_attach) = {
                let inner = state.inner.lock().await;
                let mut filter: HashSet<i64> = inner.confirmed.clone();
                for ids in inner.pending.values() {
                    filter.extend(ids.iter().copied());
                }
                (filter, state.reverse_attach.clone())
            };

            // Round-trip to the CLI for the row set. Errors → None;
            // the proxy treats that as "nothing right now."
            let result = match read_message_queue_via_ws(&reverse_attach, aih).await {
                Ok(r) => r,
                Err(_) => return None,
            };

            // Filter each row's content_ids against our ban set.
            // Drop any row whose ids are entirely banned (the row
            // is already covered); keep rows with any unseen ids
            // and surface their full content (one block per row).
            let mut new_ids: Vec<i64> = Vec::new();
            let mut blocks: Vec<Vec<ContentBlock>> = Vec::new();
            for row in result.rows {
                if row.content_ids.iter().all(|id| filter.contains(id)) {
                    continue;
                }
                new_ids.extend(row.content_ids.iter().copied());
                let row_blocks: Vec<ContentBlock> = row.rich_content.into();
                blocks.push(row_blocks);
            }

            if new_ids.is_empty() {
                return None;
            }

            // Mint a fresh token and stash this batch in pending.
            // The proxy will embed the token in the wrapper prefix
            // the agent sees; `run_agent_loop` will fish it back
            // out via regex and call `confirm()`.
            let token = uuid::Uuid::new_v4().to_string();
            {
                let mut inner = state.inner.lock().await;
                inner.pending.insert(token.clone(), new_ids);
            }
            Some(QueueRead { token, blocks })
        })
    }
}

/// Replicates `super::client::read_message_queue_via_ws` — same
/// reverse-attach round-trip, same wire shape — without taking a
/// dep on a private fn. Keeps the delegate self-contained.
async fn read_message_queue_via_ws(
    handle: &Arc<ReverseAttachHandle>,
    agent_instance_hierarchy: &str,
) -> Result<ReadMessageQueueResult, ReverseAttachError> {
    use objectiveai_sdk::client_objectiveai_mcp::{server_request, server_response};
    let rc = handle.channel();
    let request = server_request::Request {
        id: uuid::Uuid::new_v4().to_string(),
        headers: IndexMap::new(),
        payload: server_request::Payload::ReadMessageQueue(
            server_request::ReadMessageQueueRequest {
                agent_instance_hierarchy: agent_instance_hierarchy.to_string(),
            },
        ),
    };
    let rx = crate::objectiveai_mcp::send_server_request(&rc.sink, &rc.pending, request)
        .await
        .map_err(|()| ReverseAttachError::Closed)?;
    let response = match tokio::time::timeout(READ_TIMEOUT, rx).await {
        Ok(Ok(response)) => response,
        Ok(Err(_)) => return Err(ReverseAttachError::Dropped),
        Err(_) => return Err(ReverseAttachError::Timeout),
    };
    match response.payload {
        server_response::Payload::ReadMessageQueue(server_response::JsonRpcResult::Ok {
            result,
        }) => Ok(result),
        server_response::Payload::ReadMessageQueue(server_response::JsonRpcResult::Err {
            code,
            message,
            ..
        }) => Err(ReverseAttachError::CliError { code, message }),
        _ => Err(ReverseAttachError::WrongVariant),
    }
}

#[derive(Debug)]
enum ReverseAttachError {
    Closed,
    Dropped,
    Timeout,
    CliError { code: i64, message: String },
    WrongVariant,
}
