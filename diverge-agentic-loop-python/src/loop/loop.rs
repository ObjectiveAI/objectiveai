//! The loop itself: connect, catalog, run the script, run its calls.

use diverge_sdk::container_proxy::inside::Client;
use diverge_sdk::provider::endpoints::containers::agents::run::server::response;
use diverge_sdk::container_proxy::inside::agent::run::request::Message;
use diverge_sdk::provider::endpoints::containers::agents::run::server::response::{
    AgenticLoopChunk, ToolResponseChunk, user_parts,
};
use futures_util::stream::FuturesUnordered;
use futures_util::{Stream, StreamExt as _};
use rmcp::model::CallToolRequestParams;
use rmcp::{Peer, RoleClient};

use super::{Error, Item};
use crate::continuation::{Continuation, ContinuationItem};
use crate::queue::{CloseOnDrop, QUEUE};
use crate::run;
use crate::run::Feed;

/// Run the whole agentic loop, and stream what it produces.
///
/// [`run`](crate::run::run) is one run of the script; this is that
/// run in a loop, with the container's MCP proxy on the other side of
/// every tool call. The stream errs in the loop's own vocabulary —
/// which includes the run's, wholesale.
///
/// # The script reads the history as it is stored
///
/// The turn's message goes into the history FIRST, as its own `Prompt`
/// item — its MCP content blocks, untouched — and the script is fed
/// the items whole: what the database keeps and what the script
/// reads are the same array, the latest message its last element.
/// There is no request to build, and nothing to convert: what a
/// script makes of an image or a resource is the script's.
///
/// # Every turn starts fresh
///
/// Tools AND resources are re-listed before every invocation of the
/// script — the first, and each one after a turn's tool responses or
/// a prompt taken at a turn's end — so the script always sees the
/// caller's current set, and the names its calls are checked against
/// are the names it was shown.
///
/// # The turn is one answer, then its calls, all at once
///
/// The script answers when it ends, so a turn's chunks arrive
/// together. Every `assistant_tool_call` among them is checked
/// against the turn's tool list — a name the list does not carry
/// ends the run, before anything of the turn is yielded — and then
/// every call goes out at once, each on its own task, BEFORE the
/// turn's chunks are yielded: the calls run while the consumer reads
/// the answer that made them. Each response is yielded — and
/// recorded — the instant its call completes, in completion order.
/// The proxy converts every tool-level failure into a tool response
/// the script can read, so a `call_tool` error here means the MCP
/// link itself failed, and that — like any yielded error — ends the
/// stream, permanently. A stream dropped mid-drain leaves its calls
/// to finish on their own tasks; nothing reads them.
///
/// # Yield first, remember second
///
/// Every chunk of the turn is yielded and ALSO accumulated into the
/// running history: the six assistant kinds, with their `_meta`
/// stripped and tool-call fragments coalesced by id; notifications
/// are yielded and not kept. Nothing else can be in a turn — the run
/// refused it already.
///
/// # The history rests between turns, and says so
///
/// Whenever the history is AT REST — a turn's tool answers all
/// landed, or a call-less turn that said something completed — it is
/// yielded whole as [`Item::Rest`], for the caller to save. A loop
/// that dies mid-turn leaves the last rest already saved, and never
/// an unanswered call in it. An empty turn is not a rest — its
/// opening prompt hangs unanswered.
///
/// # The queue is consulted at the seams
///
/// The same two seams openrouter uses. After a turn's tool calls
/// have all answered, everything pending in [`QUEUE`] is delivered —
/// a `user` chunk per message, after the tool responses, which is
/// the position the message actually enters the conversation — and
/// each becomes its own `Prompt` item in the history. And when a turn
/// ends with NO tool calls, the queue gets a last look, atomically:
/// messages pending there open another turn, and only an empty queue
/// — closed in the same lock hold that proved it empty — lets the
/// loop end. Every delivery answers its `/enqueue`; everything still
/// pending when the stream ends, however it ends, is missed.
/// `generation` is the run's number, from
/// [`QUEUE.open`](crate::queue::Queue::open), quoted by the close.
pub async fn r#loop(
    client: &Client,
    continuation: Option<Continuation>,
    messages: Vec<Message>,
    generation: u64,
) -> Result<
    impl Stream<Item = Result<Item, Error>> + Send + Unpin + use<>,
    Error,
> {
    // First, before anything can fail or unwind: the guard that
    // closes the queue however this run ends. From here on there is
    // no exit — Err return, panic, or the stream below dying at any
    // age, polled or not — that leaves the queue open and a pending
    // message waiting on a loop that will never look. It carries the
    // run's number, so a close that runs late cannot touch the next.
    let close = CloseOnDrop(generation);

    // The MCP session: the proxy beside us, dialed by the client the
    // first time it is asked and held for the program's life.
    let peer = client.mcp_peer().await.map_err(Error::Connect)?.clone();

    let mut items: Vec<ContinuationItem> =
        continuation.map(|history| history.0).unwrap_or_default();
    for message in &messages {
        items.push(ContinuationItem::Prompt(message.content.clone()));
    }

    // Turn one, run here so a failure to start is this function's
    // `Err` and never a stream's leading item.
    let turn = turn(&peer, &items).await?;

    // The guard moves INTO the stream — but as a captured local, not
    // a body-created one: a generator's body runs on first poll, so
    // a guard born inside it would not exist in the window where the
    // stream is dropped unpolled — and that window is exactly the
    // kind of nanosecond it guards. Captured, it dies with the
    // stream, polled or not; the graceful path has already closed
    // the queue by then, and every close after the first is a no-op.
    Ok(Box::pin(async_stream::stream! {
        let _close = close;
        let peer = peer;
        let mut items = items;
        let mut turn = turn;

        // The messages the run started on, as the stream's first
        // chunks: their parts, each under its key, before the script
        // says a word — the first run is already made, so a start
        // that failed was this function's `Err` and never got here.
        for message in messages {
            for chunk in user_parts(&message.key, message.content) {
                yield Ok(Item::Chunk(chunk));
            }
        }

        loop {
            let Turn { chunks, calls } = turn;

            // What the history keeps of the turn, coalesced and
            // stripped; whether the script said anything at all.
            let mut kept: Vec<AgenticLoopChunk> = Vec::new();
            for chunk in &chunks {
                accumulate(&mut kept, chunk);
            }
            let spoke = !kept.is_empty();
            items.extend(kept.into_iter().map(ContinuationItem::Chunk));

            // Every call at once, on its own task, before a single
            // chunk is yielded: the calls run while the answer that
            // made them is read.
            let mut pending = FuturesUnordered::new();
            for call in calls {
                let peer = peer.clone();
                pending.push(tokio::spawn(async move {
                    let mut params = CallToolRequestParams::new(call.name);
                    // Arguments that never became a JSON object are
                    // sent as none; the tool's refusal comes back as
                    // a tool response, which is the script's to read.
                    params.arguments = serde_json::from_str(&call.arguments).ok();
                    (call.id, peer.call_tool(params).await)
                }));
            }
            for chunk in chunks {
                yield Ok(Item::Chunk(chunk));
            }

            if pending.is_empty() {
                // No calls: the script may be done. A turn that said
                // something is at rest, and the rest is yielded
                // BEFORE the last look, so a prompt delivered here
                // stays out of it. Then the look, atomic: an empty
                // queue is CLOSED in the same lock hold that proved
                // it empty, and the loop ends; messages pending open
                // another turn, each its own user parts and its own
                // Prompt item.
                if spoke {
                    yield Ok(Item::Rest(Continuation(items.clone())));
                }
                let taken = QUEUE.take_or_close().await;
                if taken.is_empty() {
                    return;
                }
                for message in taken {
                    for chunk in user_parts(&message.key, message.content.clone()) {
                        yield Ok(Item::Chunk(chunk));
                    }
                    items.push(ContinuationItem::Prompt(message.content.clone()));
                    message.deliver();
                }
            } else {
                // Every answer the moment it lands.
                while let Some(joined) = pending.next().await {
                    let (id, result) = match joined {
                        Ok(answer) => answer,
                        Err(error) => {
                            yield Err(Error::Join(error));
                            return;
                        }
                    };
                    match result {
                        Ok(result) => {
                            let chunk = ToolResponseChunk {
                                r#type: Default::default(),
                                parent_tool_call_id: None,
                                id,
                                inner: result,
                            };
                            let mut kept = AgenticLoopChunk::ToolResponse(chunk.clone());
                            strip(&mut kept);
                            items.push(ContinuationItem::Chunk(kept));
                            yield Ok(Item::Chunk(AgenticLoopChunk::ToolResponse(chunk)));
                        }
                        Err(error) => {
                            yield Err(Error::CallTool(error));
                            return;
                        }
                    }
                }
                // The tool seam: everything enqueued while the tools
                // ran is delivered here, after the answers and before
                // the script runs again — the position it truly
                // enters the conversation.
                for message in QUEUE.take().await {
                    for chunk in user_parts(&message.key, message.content.clone()) {
                        yield Ok(Item::Chunk(chunk));
                    }
                    items.push(ContinuationItem::Prompt(message.content.clone()));
                    message.deliver();
                }
                // Every answer is in and the seam's deliveries with
                // them: the history is at rest.
                yield Ok(Item::Rest(Continuation(items.clone())));
            }

            // The next turn: fresh tools, fresh resources, the script
            // run again over the history — which carries the
            // conversation now, the prompts included.
            turn = match self::turn(&peer, &items).await {
                Ok(turn) => turn,
                Err(error) => {
                    yield Err(error);
                    return;
                }
            };
        }
    }))
}

/// One turn, run and checked: the script's chunks, and the calls
/// among them.
struct Turn {
    /// What the script answered, in its order.
    chunks: Vec<AgenticLoopChunk>,
    /// The tool calls in it, coalesced by id, every name known.
    calls: Vec<Call>,
}

/// One tool call the script made.
struct Call {
    /// The call's id, the script's own.
    id: String,
    /// The tool, by name.
    name: String,
    /// The arguments, as the JSON text the fragments add up to.
    arguments: String,
}

/// List, run, assemble, check.
///
/// The two listings go out together — the paginating helpers, so a
/// cursor never truncates a set — then the script runs over the
/// history and both, and its calls are gathered by id across the
/// whole answer: interleaved fragments correlate by id, not
/// adjacency. Every call's name is then looked up in the very list
/// the script was fed; one it does not carry is
/// [`Error::UnknownTool`], and nothing of the turn goes out.
async fn turn(peer: &Peer<RoleClient>, items: &[ContinuationItem]) -> Result<Turn, Error> {
    let (tools, resources) =
        futures_util::future::join(peer.list_all_tools(), peer.list_all_resources()).await;
    let tools = tools.map_err(Error::ListTools)?;
    let resources = resources.map_err(Error::ListResources)?;

    let chunks = run::run(&Feed {
        input: items,
        tools: &tools,
        resources: &resources,
    })
    .await?;

    let mut calls: Vec<Call> = Vec::new();
    for chunk in &chunks {
        if let AgenticLoopChunk::AssistantToolCall(chunk) = chunk {
            match calls.iter_mut().find(|call| call.id == chunk.id) {
                Some(call) => {
                    if let Some(fragment) = &chunk.arguments {
                        call.arguments.push_str(fragment);
                    }
                }
                None => calls.push(Call {
                    id: chunk.id.clone(),
                    name: chunk.name.clone(),
                    arguments: chunk.arguments.clone().unwrap_or_default(),
                }),
            }
        }
    }
    for call in &calls {
        if !tools.iter().any(|tool| tool.name.as_ref() == call.name) {
            return Err(Error::UnknownTool {
                id: call.id.clone(),
                name: call.name.clone(),
            });
        }
    }

    Ok(Turn { chunks, calls })
}

/// Keep what the history keeps.
///
/// The six assistant kinds accumulate, `_meta` stripped — the yielded
/// chunk keeps its provenance, the recorded one does not — and the
/// fragmented kinds coalesce through the SDK's own [`response::push`]:
/// the history keeps what was said, not how it was cut.
/// Notifications say nothing the conversation replays, and are not
/// kept. The kinds a script may not say never reach here — the run
/// refused them — and the arms are for the match's completeness.
fn accumulate(turn: &mut Vec<AgenticLoopChunk>, chunk: &AgenticLoopChunk) {
    if matches!(
        chunk,
        AgenticLoopChunk::Usage(_)
            | AgenticLoopChunk::Notification(_)
            | AgenticLoopChunk::UserTextContent(_)
            | AgenticLoopChunk::UserImageContent(_)
            | AgenticLoopChunk::UserAudioContent(_)
            | AgenticLoopChunk::UserResource(_)
            | AgenticLoopChunk::UserResourceLink(_)
    ) {
        return;
    }
    let mut kept = chunk.clone();
    strip(&mut kept);
    response::push(turn, kept);
}

/// Remove the `_meta` a chunk carries, wherever its kind keeps it.
/// The history stores what was said, not where it came from. (A
/// chunk's `parent_tool_call_id` is left alone: that is structure —
/// which thread said it — not provenance; this loop never sets it.)
fn strip(chunk: &mut AgenticLoopChunk) {
    match chunk {
        AgenticLoopChunk::AssistantReasoning(chunk) => {
            chunk.inner.meta = None;
        }
        AgenticLoopChunk::AssistantTextContent(chunk) => {
            chunk.inner.meta = None;
        }
        AgenticLoopChunk::AssistantImageContent(chunk) => {
            chunk.inner.meta = None;
        }
        AgenticLoopChunk::AssistantAudioContent(chunk) => {
            chunk.inner.meta = None;
        }
        AgenticLoopChunk::AssistantRefusal(chunk) => {
            chunk.inner.meta = None;
        }
        AgenticLoopChunk::AssistantToolCall(chunk) => {
            chunk.meta = None;
        }
        AgenticLoopChunk::ToolResponse(chunk) => {
            chunk.inner.meta = None;
        }
        AgenticLoopChunk::UserTextContent(chunk) => {
            chunk.inner.meta = None;
        }
        AgenticLoopChunk::UserImageContent(chunk) => {
            chunk.inner.meta = None;
        }
        AgenticLoopChunk::UserAudioContent(chunk) => {
            chunk.inner.meta = None;
        }
        AgenticLoopChunk::UserResource(chunk) => {
            chunk.inner.meta = None;
        }
        AgenticLoopChunk::UserResourceLink(chunk) => {
            chunk.inner.meta = None;
        }
        AgenticLoopChunk::Usage(chunk) => {
            chunk.meta = None;
        }
        AgenticLoopChunk::Notification(chunk) => {
            chunk.meta = None;
        }
    }
}
