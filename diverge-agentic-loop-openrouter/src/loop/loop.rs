//! The loop itself: connect, learn the tools, run the turns.

use diverge_container_proxy_sdk::Client;
use diverge_provider_sdk::endpoints::containers::agents::agent::openrouter;
use diverge_provider_sdk::shared::containers::run_loop::response;
use diverge_provider_sdk::shared::containers::run_loop::response::{
    AgenticLoopChunk, ToolResponseChunk, UserChunk,
};
use futures_util::stream::FuturesUnordered;
use futures_util::{Stream, StreamExt as _};
use rmcp::model::CallToolRequestParams;
use rmcp::{Peer, RoleClient};

use super::{Error, Item};
use crate::continuation::{Continuation, ContinuationItem};
use crate::fetch;
use crate::queue::{CloseOnDrop, QUEUE};
use crate::request::Tool;

/// Run the whole agentic loop, and stream what it produces.
///
/// [`fetch`](crate::fetch::fetch) is one OpenRouter call; this is
/// that call in a loop, with the container's MCP proxy on the other
/// side of every tool call. The stream is the same shape as fetch's,
/// erring in the loop's own vocabulary — which includes fetch's,
/// wholesale.
///
/// # Yield first, remember second
///
/// Every chunk a turn produces is yielded the moment it arrives —
/// nothing waits on anything — and is ALSO accumulated into the
/// running history: the six assistant kinds, with their `_meta`
/// stripped and tool-call fragments coalesced by id; usage and
/// notifications are yielded and not kept. The history is the
/// original continuation's items, then the turn-one prompt, then
/// everything each turn said and every tool's answer.
///
/// # Tool calls run in parallel, and answers land as they finish
///
/// After a clean turn, every tool call goes out at once, and each
/// response is yielded — and recorded — the instant ITS call
/// completes, in completion order, waiting for none of its siblings.
/// The proxy converts every tool-level failure into a tool response
/// the model can read, so a `call_tool` error here means the MCP link
/// itself failed, and that — like any yielded error — ends the
/// stream, permanently.
///
/// # Every turn starts fresh
///
/// Tools are re-listed and the request is rebuilt for every turn:
/// the tool set is the caller's and may have changed while the tools
/// ran.
///
/// # The history rests between turns, and says so
///
/// Whenever the history is AT REST — a turn's tool answers all
/// landed, or a call-less turn that said something completed — it is
/// yielded whole as [`Item::Rest`], for the caller to save. That is
/// the old salvage made continuous: a loop that dies mid-turn leaves
/// the last rest already saved, and never an unanswered call or a
/// partial response in it. An empty turn is not a rest — its opening
/// prompt hangs unanswered.
///
/// # The queue is consulted at the seams
///
/// The same two seams Claude Code uses. After a turn's tool calls
/// have all answered, everything pending in [`QUEUE`] is delivered —
/// a `user` chunk per message, after the tool responses, which is
/// the position the message actually enters the conversation — and
/// each becomes its own `Prompt` item in the history, unjoined and
/// unwrapped: how delivered prompts sit inside a tool message is the
/// request builder's derivation, not the continuation's business.
/// And when a turn ends with NO tool calls, the queue gets a last
/// look, atomically: messages pending there open another turn, and
/// only an empty queue — closed in the same lock hold that proved it
/// empty — lets the loop end. Every delivery answers its `/enqueue`;
/// everything still pending when the stream ends, however it ends,
/// is missed.
pub async fn r#loop(
    client: &Client,
    api_key: &str,
    agent: openrouter::Agent,
    continuation: Option<Continuation>,
    prompt: String,
) -> Result<
    impl Stream<Item = Result<Item, Error>> + Send + Unpin + use<>,
    Error,
> {
    // First, before anything can fail or unwind: the guard that
    // closes the queue however this run ends. From here on there is
    // no exit — Err return, panic, or the stream below dying at any
    // age, polled or not — that leaves the queue open and a pending
    // message waiting on a loop that will never look.
    let close = CloseOnDrop;

    // The MCP session: the proxy beside us, dialed by the client the
    // first time it is asked and held for the program's life.
    let peer = client.mcp_peer().await.map_err(Error::Connect)?.clone();

    let mut items: Vec<ContinuationItem> =
        continuation.map(|history| history.0).unwrap_or_default();

    // Turn one, started here so a failure to start is this function's
    // `Err` and never a stream's leading item.
    let tools = list(&peer).await?;
    let stream = fetch::fetch(
        api_key,
        agent.clone(),
        Some(Continuation(items.clone())),
        prompt.clone(),
        Some(tools),
    )
    .await?;
    items.push(ContinuationItem::Prompt(prompt));

    let api_key = api_key.to_string();
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
        let mut stream = stream;
        let mut items = items;

        loop {
            // Drain the turn: yield everything immediately, keep what
            // the history keeps.
            let mut turn: Vec<AgenticLoopChunk> = Vec::new();
            while let Some(item) = stream.next().await {
                match item {
                    Ok(chunk) => {
                        accumulate(&mut turn, &chunk);
                        yield Ok(Item::Chunk(chunk));
                    }
                    Err(error) => {
                        yield Err(Error::Fetch(error));
                        return;
                    }
                }
            }

            // The turn's calls, assembled by id across the whole turn
            // — interleaved fragments correlate by id, not adjacency.
            let mut calls: Vec<(String, String, String)> = Vec::new();
            for chunk in &turn {
                if let AgenticLoopChunk::AssistantToolCall(chunk) = chunk {
                    match calls.iter_mut().find(|(id, ..)| *id == chunk.id) {
                        Some((_, _, arguments)) => {
                            if let Some(fragment) = &chunk.arguments {
                                arguments.push_str(fragment);
                            }
                        }
                        None => calls.push((
                            chunk.id.clone(),
                            chunk.name.clone(),
                            chunk.arguments.clone().unwrap_or_default(),
                        )),
                    }
                }
            }
            // Whether the model actually said anything the history
            // keeps — an empty turn leaves its opening prompt
            // unanswered, and an unanswered prompt is no resting
            // place.
            let spoke = !turn.is_empty();
            items.extend(turn.into_iter().map(ContinuationItem::Chunk));

            if calls.is_empty() {
                // No calls: the model may be done. A turn that said
                // something is at rest, and the rest is yielded
                // BEFORE the last look: a prompt delivered here would
                // trail the assistant's answer with nothing to fold
                // onto, so it stays out of this rest — and a later
                // turn that dies loses its place, accepted. Then the
                // look, atomic: an empty queue is CLOSED in the same
                // lock hold that proved it empty, and the loop ends;
                // messages pending open another turn, each its own
                // user chunk and its own Prompt item.
                if spoke {
                    yield Ok(Item::Rest(Continuation(items.clone())));
                }
                let taken = QUEUE.take_or_close().await;
                if taken.is_empty() {
                    return;
                }
                for message in taken {
                    yield Ok(Item::Chunk(AgenticLoopChunk::User(UserChunk {
                        r#type: Default::default(),
                        prompt: message.prompt.clone(),
                        meta: None,
                    })));
                    items.push(ContinuationItem::Prompt(message.prompt.clone()));
                    message.deliver();
                }
            } else {
                // Every call at once; every answer the moment it lands.
                let mut pending = FuturesUnordered::new();
                for (id, name, arguments) in calls {
                    let peer = peer.clone();
                    pending.push(async move {
                        let mut params = CallToolRequestParams::new(name);
                        // Arguments that never became a JSON object are
                        // sent as none; the tool's refusal comes back as a
                        // tool response, which is the model's to read.
                        params.arguments = serde_json::from_str(&arguments).ok();
                        (id, peer.call_tool(params).await)
                    });
                }
                while let Some((id, result)) = pending.next().await {
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
                // the model speaks again — the position Claude Code
                // gives it, and the position it truly enters the
                // conversation.
                for message in QUEUE.take().await {
                    yield Ok(Item::Chunk(AgenticLoopChunk::User(UserChunk {
                        r#type: Default::default(),
                        prompt: message.prompt.clone(),
                        meta: None,
                    })));
                    items.push(ContinuationItem::Prompt(message.prompt.clone()));
                    message.deliver();
                }
                // Every answer is in and the seam's deliveries with
                // them: the history is at rest. A resume from here
                // ends on the responses — or on prompts that FOLD ONTO
                // them at request-building time, which is a valid
                // resume and keeps the delivered messages delivered.
                yield Ok(Item::Rest(Continuation(items.clone())));
            }

            // The next turn: fresh tools, fresh request, no new
            // prompt — the history carries the conversation now.
            let tools = match list(&peer).await {
                Ok(tools) => tools,
                Err(error) => {
                    yield Err(error);
                    return;
                }
            };
            stream = match fetch::fetch(
                &api_key,
                agent.clone(),
                Some(Continuation(items.clone())),
                String::new(),
                Some(tools),
            )
            .await
            {
                Ok(stream) => stream,
                Err(error) => {
                    yield Err(Error::Fetch(error));
                    return;
                }
            };
        }
    }))
}

/// The tools the caller's servers offer, through the proxy's one
/// listing — the paginating helper, so a cursor never truncates the
/// set.
async fn list(peer: &Peer<RoleClient>) -> Result<Vec<Tool>, Error> {
    Ok(peer
        .list_all_tools()
        .await
        .map_err(Error::ListTools)?
        .into_iter()
        .map(Tool::new)
        .collect())
}

/// Keep what the history keeps.
///
/// The six assistant kinds accumulate, `_meta` stripped — the yielded
/// chunk keeps its provenance, the recorded one does not — and the
/// streamed kinds coalesce through the SDK's own [`response::push`]:
/// the history keeps what was said, not how it was cut. Usage and
/// notifications say nothing the conversation replays, and are not
/// kept; user chunks are not either — a delivered message enters the
/// history as its own `Prompt` item at the seam that delivered it,
/// not as a chunk.
fn accumulate(turn: &mut Vec<AgenticLoopChunk>, chunk: &AgenticLoopChunk) {
    if matches!(
        chunk,
        AgenticLoopChunk::Usage(_)
            | AgenticLoopChunk::Notification(_)
            | AgenticLoopChunk::User(_)
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
        AgenticLoopChunk::User(chunk) => {
            chunk.meta = None;
        }
        AgenticLoopChunk::Usage(chunk) => {
            chunk.meta = None;
        }
        AgenticLoopChunk::Notification(chunk) => {
            chunk.meta = None;
        }
    }
}
