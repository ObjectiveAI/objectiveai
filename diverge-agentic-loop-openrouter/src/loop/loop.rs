//! The loop itself: connect, learn the tools, run the turns.

use diverge_provider_sdk::endpoints::agentic_loop::run::client::request::agent::openrouter;
use diverge_provider_sdk::endpoints::agentic_loop::run::server::response;
use diverge_provider_sdk::endpoints::agentic_loop::run::server::response::{
    AgenticLoopChunk, ContinuationChunk, ToolResponseChunk, UserChunk,
};
use futures_util::stream::FuturesUnordered;
use futures_util::{Stream, StreamExt as _};
use rmcp::ServiceExt as _;
use rmcp::model::CallToolRequestParams;
use rmcp::transport::StreamableHttpClientTransport;

use super::Error;
use crate::continuation::{Continuation, ContinuationItem};
use crate::fetch;
use crate::queue::{CloseOnDrop, QUEUE};
use crate::request::Tool;

/// Where the in-container MCP proxy serves: its hard-coded port —
/// the Container specification's MCP port — and rmcp's conventional
/// path, on loopback.
const MCP_PROXY: &str = "http://localhost:8081/mcp";

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
/// stripped and tool-call fragments coalesced by id; usage,
/// notifications and continuations are yielded and not kept. The
/// history is the original continuation's items, then the turn-one
/// prompt, then everything each turn said and every tool's answer.
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
/// empty — lets the continuation be minted. Every delivery answers
/// its `/enqueue`; everything still pending when the stream ends,
/// however it ends, is missed.
pub async fn r#loop(
    api_key: &str,
    agent: openrouter::Agent,
    continuation: Option<Continuation>,
    prompt: Vec<rmcp::model::ContentBlock>,
) -> Result<
    impl Stream<Item = Result<AgenticLoopChunk, Error>> + Send + Unpin + use<>,
    Error,
> {
    // The MCP connection: the proxy beside us, dialled once per run
    // and owned by the generator for the life of the stream.
    let mcp = ()
        .serve(StreamableHttpClientTransport::from_uri(MCP_PROXY))
        .await?;

    let mut items: Vec<ContinuationItem> =
        continuation.map(|history| history.0).unwrap_or_default();

    // Turn one, started here so a failure to start is this function's
    // `Err` and never a stream's leading item.
    let tools = list(&mcp).await?;
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
    Ok(Box::pin(async_stream::stream! {
        let mcp = mcp;
        let mut stream = stream;
        let mut items = items;
        // However this stream ends — continuation minted, error
        // yielded, or dropped mid-run — the queue ends up closed and
        // every pending message answered; the graceful path has
        // already closed it, and this makes the other paths honest.
        let _close = CloseOnDrop;

        loop {
            // Drain the turn: yield everything immediately, keep what
            // the history keeps.
            let mut turn: Vec<AgenticLoopChunk> = Vec::new();
            while let Some(item) = stream.next().await {
                match item {
                    Ok(chunk) => {
                        accumulate(&mut turn, &chunk);
                        yield Ok(chunk);
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
            items.extend(turn.into_iter().map(ContinuationItem::Chunk));

            // No calls: the model may be done — but the queue gets
            // the last look, and the look is atomic: an empty queue
            // is CLOSED in the same lock hold that proved it empty,
            // so no enqueue can land between this decision and the
            // end. Only then is the loop's last word the
            // continuation — the whole history, tokenized, so a
            // later request can pick up exactly here.
            if calls.is_empty() {
                let taken = QUEUE.take_or_close().await;
                if taken.is_empty() {
                    match Continuation(items).tokenize() {
                        Ok(token) => {
                            yield Ok(AgenticLoopChunk::Continuation(
                                ContinuationChunk {
                                    r#type: Default::default(),
                                    continuation: token,
                                    meta: None,
                                },
                            ));
                        }
                        Err(error) => {
                            yield Err(Error::Tokenize(error));
                        }
                    }
                    return;
                }
                // Messages pending: they open another turn, each its
                // own user chunk and its own Prompt item.
                for message in taken {
                    yield Ok(AgenticLoopChunk::User(UserChunk {
                        r#type: Default::default(),
                        prompt: message.prompt.clone(),
                        meta: None,
                    }));
                    items.push(ContinuationItem::Prompt(vec![
                        rmcp::model::ContentBlock::text(
                            message.prompt.clone(),
                        ),
                    ]));
                    message.deliver();
                }
            } else {
                // Every call at once; every answer the moment it
                // lands.
                let mut pending = FuturesUnordered::new();
                for (id, name, arguments) in calls {
                    let peer = mcp.peer().clone();
                    pending.push(async move {
                        let mut params = CallToolRequestParams::new(name);
                        // Arguments that never became a JSON object
                        // are sent as none; the tool's refusal comes
                        // back as a tool response, which is the
                        // model's to read.
                        params.arguments =
                            serde_json::from_str(&arguments).ok();
                        (id, peer.call_tool(params).await)
                    });
                }
                while let Some((id, result)) = pending.next().await {
                    match result {
                        Ok(result) => {
                            let chunk = ToolResponseChunk {
                                r#type: Default::default(),
                                id,
                                inner: result,
                            };
                            let mut kept =
                                AgenticLoopChunk::ToolResponse(chunk.clone());
                            strip(&mut kept);
                            items.push(ContinuationItem::Chunk(kept));
                            yield Ok(AgenticLoopChunk::ToolResponse(chunk));
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
                    yield Ok(AgenticLoopChunk::User(UserChunk {
                        r#type: Default::default(),
                        prompt: message.prompt.clone(),
                        meta: None,
                    }));
                    items.push(ContinuationItem::Prompt(vec![
                        rmcp::model::ContentBlock::text(
                            message.prompt.clone(),
                        ),
                    ]));
                    message.deliver();
                }
            }

            // The next turn: fresh tools, fresh request, no new
            // prompt — the history carries the conversation now.
            let tools = match list(&mcp).await {
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
                Vec::new(),
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
async fn list(
    mcp: &rmcp::service::RunningService<rmcp::RoleClient, ()>,
) -> Result<Vec<Tool>, Error> {
    Ok(mcp
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
/// the history keeps what was said, not how it was cut. Usage,
/// notifications and continuations say nothing the conversation
/// replays, and are not kept; user chunks are not either, because
/// this upstream never produces one — the queue is unwired here.
fn accumulate(turn: &mut Vec<AgenticLoopChunk>, chunk: &AgenticLoopChunk) {
    if matches!(
        chunk,
        AgenticLoopChunk::Usage(_)
            | AgenticLoopChunk::Notification(_)
            | AgenticLoopChunk::Continuation(_)
            | AgenticLoopChunk::User(_)
    ) {
        return;
    }
    let mut kept = chunk.clone();
    strip(&mut kept);
    response::push(turn, kept);
}

/// Remove the `_meta` a chunk carries, wherever its kind keeps it.
/// The history stores what was said, not where it came from.
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
        AgenticLoopChunk::Continuation(chunk) => {
            chunk.meta = None;
        }
    }
}
