//! `agents spawn` — in-process chunk-or-id streaming handler.
//!
//! No subprocess. Spawns drive the SDK streaming WS connection
//! directly inside the cli process and yield each chunk as it
//! arrives. Per-chunk: claim a process-owned file for every new
//! `agent_instance_hierarchy` and upgrade any PENDING tags that
//! map to it. End-of-stream: check whether any seen hierarchy has
//! undelivered `message_queue` rows; if so, recurse with the same
//! params but the latest captured continuation. Yields chunks
//! straight through as they arrive — restart passes flow into the
//! same output stream.
//!
//! - `dangerous_advanced.stream = Some(true)` → real streaming
//!   path: emit `ResponseItem::Id(<agent_instance_hierarchy>)` on
//!   first chunk, then one `ResponseItem::Chunk` per chunk, across
//!   any number of restart passes, until no seen hierarchy has
//!   pending messages.
//! - `dangerous_advanced.stream = None | Some(false)` → recurse
//!   into self with `stream = Some(true)`, yield only the
//!   `ResponseItem::Id`, then drive the rest of the stream to EOF
//!   silently in-process (so the message-queue restart logic still
//!   runs to completion).
//!
//! `params.stream` on the wire is always `Some(true)`; the
//! `dangerous_advanced.stream` setting only controls cli-side
//! output.

use std::collections::HashMap;
use std::future::Future;
use std::path::PathBuf;
use std::pin::Pin;

use futures::Stream;
use futures::StreamExt;
use objectiveai_sdk::agent::InlineAgentBaseWithFallbacksOrRemoteCommitOptional;
use objectiveai_sdk::agent::completions::message::{
    Message, RichContent, UserMessage,
};
use objectiveai_sdk::agent::completions::request::AgentCompletionCreateParams;
use objectiveai_sdk::cli::command::agents::instances::spawn::{
    AgentSpec, Request, RequestDangerousAdvanced, RequestPrompt, ResponseItem,
};

use crate::context::Context;
use crate::db;
use crate::error::Error;
use crate::agent_registry::AgentInstanceRegistry;

type ItemStream = Pin<Box<dyn Stream<Item = Result<ResponseItem, Error>> + Send>>;

pub fn execute<'a>(
    ctx: &'a Context,
    request: Request,
) -> Pin<Box<dyn Future<Output = Result<ItemStream, Error>> + Send + 'a>> {
    Box::pin(async move {
        let want_stream = request
            .dangerous_advanced
            .as_ref()
            .and_then(|a| a.stream)
            .unwrap_or(false);
        if want_stream {
            execute_streaming(ctx, request).await
        } else {
            // Recurse with stream=true; emit only the first
            // ResponseItem::Id, then drive the rest in-process to
            // EOF so the restart logic still runs.
            let mut req = request;
            match req.dangerous_advanced.as_mut() {
                Some(adv) => adv.stream = Some(true),
                None => {
                    req.dangerous_advanced =
                        Some(RequestDangerousAdvanced { stream: Some(true) })
                }
            }
            let inner = execute(ctx, req).await?;
            Ok(yield_id_then_drain(inner))
        }
    })
}

/// Wraps an inner stream that emits `ResponseItem::Id` followed by
/// `ResponseItem::Chunk`s. The outer stream yields the first
/// `ResponseItem::Id` it sees, then continues to drive the inner
/// stream to EOF without yielding anything else. Errors from the
/// inner stream are bubbled exactly once.
fn yield_id_then_drain(mut inner: ItemStream) -> ItemStream {
    Box::pin(async_stream::try_stream! {
        let mut id_yielded = false;
        while let Some(item) = inner.next().await {
            let item = item?;
            if !id_yielded {
                if let ResponseItem::Id(_) = &item {
                    id_yielded = true;
                    yield item;
                }
                // Pre-Id chunks are not expected; drop them.
            }
            // Post-Id items drain silently.
        }
    })
}

async fn execute_streaming(
    ctx: &Context,
    request: Request,
) -> Result<ItemStream, Error> {
    let mut messages = resolve_prompt(request.prompt)?;

    // Drain the queue once before the first pass. Two-rule
    // predicate (see `db::message_queue::drain_for_spawn`):
    //   1. queue items addressed to `request.agent_tag` directly
    //   2. queue items addressed to any PENDING tag whose
    //      `(parent, agent_full_id)` matches this spawn.
    let agent_full_id_top = resolve_agent_full_id(ctx, &request.agent).await?;
    let drained = db::message_queue::drain_for_spawn(
        &ctx.db,
        &ctx.config.agent_instance_hierarchy,
        &agent_full_id_top,
        request.agent_tag.as_deref(),
    )
    .await?;
    if !drained.is_empty() {
        let prepended = crate::command::message_queue_drain::join_with_separator(
            drained.iter().map(|d| d.content.clone()).collect(),
        );
        messages.insert(
            0,
            Message::User(UserMessage {
                content: prepended,
                name: None,
            }),
        );
    }

    let agent = resolve_agent(ctx, request.agent).await?;
    let agent_tag = request.agent_tag.clone();
    let agents_dir = ctx
        .filesystem
        .base_dir()
        .join("instances")
        .join("agents");

    let params = AgentCompletionCreateParams {
        messages,
        provider: None,
        agent,
        response_format: None,
        seed: request.seed,
        stream: Some(true),
        continuation: None,
    };

    // Run the multi-pass driver as an async-stream so chunks flow
    // straight through to the consumer (no Vec collection).
    let ctx_clone = ctx.clone();
    let driver = run_multi_pass(ctx_clone, params, agent_tag, agents_dir);

    // Peek the first item — on error/empty BEFORE any item, restore
    // the drained queue rows (the spawn never effectively ran).
    let mut tail: ItemStream = Box::pin(driver);
    match tail.as_mut().next().await {
        Some(Ok(first)) => Ok(Box::pin(
            objectiveai_sdk::cli::command::StreamOnce::new(Ok(first)).chain(tail),
        )),
        Some(Err(e)) => {
            let r = db::message_queue::re_enqueue(&ctx.db, drained).await;
            Err(crate::command::message_queue_drain::combine_drain_failure(
                e, r,
            ))
        }
        None => {
            let r = db::message_queue::re_enqueue(&ctx.db, drained).await;
            Err(crate::command::message_queue_drain::combine_drain_failure(
                Error::EmptyStream,
                r,
            ))
        }
    }
}

/// Drives one or more stream passes until no seen hierarchy has
/// pending `message_queue` items. Each pass opens a fresh WS
/// stream + log writer + MCP server + conduit; the
/// [`AgentInstanceRegistry`] persists across passes so an agent's
/// process-owned claim file stays held for the whole spawn
/// lifetime, not per-pass.
fn run_multi_pass(
    ctx: Context,
    initial_params: AgentCompletionCreateParams,
    agent_tag: Option<String>,
    agents_dir: PathBuf,
) -> impl Stream<Item = Result<ResponseItem, Error>> + Send {
    async_stream::try_stream! {
        let mut registry = AgentInstanceRegistry::new(agents_dir)
            .map_err(|e| Error::Instance(format!(
                "failed to open agent claim registry: {e}"
            )))?;
        let mut params = initial_params;
        let mut emitted_id = false;
        let mut first_pass = true;

        loop {
            // Per-pass resources. New WS connection, new log writer,
            // new conduit + MCP server. The registry survives across
            // passes (see above).
            let mcp_server =
                crate::mcp_server::spawn(ctx.clone());
            let conduit =
                crate::api::conduit::ConduitMcpHandler::new(
                    mcp_server,
                    ctx.clone(),
                );
            let mut log_writer = crate::db::logs::write_agent_completion(
                &ctx.db, &params,
            )
            .map_err(|e| Error::Instance(format!(
                "failed to build agent-completion log writer: {e}"
            )))?;

            let (sdk_stream, notifier) =
                objectiveai_sdk::agent::completions::create_agent_completion_streaming(
                    &ctx.http,
                    params.clone(),
                    conduit.clone(),
                )
                .await
                .map_err(|e| Error::Instance(format!(
                    "failed to open agent-completion stream: {e}"
                )))?;
            conduit.install_notifier(notifier);

            let mut sdk_stream = Box::pin(sdk_stream);
            let mut seen: HashMap<String, String> = HashMap::new();
            let mut last_continuation: Option<String> = None;

            while let Some(item) = sdk_stream.next().await {
                let chunk = item.map_err(|e| {
                    Error::Instance(format!("agent stream item error: {e}"))
                })?;

                // First chunk of the FIRST pass: bind the explicit
                // `agent_tag` (if any) to the spawn's primary
                // hierarchy. Subsequent passes share the same root,
                // so we only do this once.
                if !emitted_id {
                    let id = chunk.agent_instance_hierarchy.clone();
                    if first_pass {
                        if let Some(tag) = &agent_tag {
                            let _ = crate::db::tags::upsert_bound(
                                &ctx.db, tag, &id,
                            )
                            .await;
                        }
                    }
                    emitted_id = true;
                    yield ResponseItem::Id(id);
                }

                // Latest continuation seen on the wire — what we
                // use to restart if any pending messages turn up at
                // EOF. Only the terminal chunk usually carries one.
                if let Some(c) = chunk.continuation.as_deref() {
                    last_continuation = Some(c.to_string());
                }

                // Per-chunk hook: for every *new*
                // (agent_instance_hierarchy, agent_full_id) pair,
                // claim the process-owned lock file and upgrade
                // any PENDING tags that match. Both are best-effort
                // (the file may already be claimed by another live
                // process; the upgrade is a no-op if no matching
                // rows exist).
                let hier = chunk.agent_instance_hierarchy.as_str();
                let full_id = chunk.agent_full_id.as_str();
                if !seen.contains_key(hier) {
                    registry.observe(hier);
                    let parent = crate::db::tags::parent_of(hier);
                    let _ = crate::db::message_queue::upgrade_and_check_pending(
                        &ctx.db, hier, parent, full_id,
                    )
                    .await;
                    seen.insert(hier.to_string(), full_id.to_string());
                }

                // Log + forward.
                log_writer
                    .write(&chunk)
                    .await
                    .map_err(|e| Error::Instance(format!(
                        "log writer error: {e}"
                    )))?;
                yield ResponseItem::Chunk(chunk);
            }

            log_writer.finalize().await.map_err(|e| {
                Error::Instance(format!("log writer finalize: {e}"))
            })?;
            drop(sdk_stream);
            drop(conduit);
            first_pass = false;

            // End-of-pass: re-run the combined upgrade-and-check
            // against every seen (hierarchy, agent_full_id) pair.
            // Doing the upgrade pass again here matters — tags
            // that became PENDING during the stream are upgraded
            // now so the message_queue lookup can see anything
            // routed via newly-bound tags.
            let mut any_pending = false;
            for (hier, full_id) in &seen {
                let parent = crate::db::tags::parent_of(hier);
                let pending = crate::db::message_queue::upgrade_and_check_pending(
                    &ctx.db, hier, parent, full_id,
                )
                .await
                .unwrap_or(false);
                if pending {
                    any_pending = true;
                    // No `break` — every iteration runs the upgrade
                    // for its side effect. The bool tracks whether
                    // ANY of them tripped pending.
                }
            }

            if !any_pending {
                break;
            }

            // Restart with the latest continuation; same agent,
            // same messages, same seed, same tag-binding (already
            // done above).
            params.continuation = last_continuation;
        }
    }
}

pub(crate) fn resolve_prompt(
    prompt: Option<RequestPrompt>,
) -> Result<Vec<Message>, Error> {
    let Some(prompt) = prompt else {
        return Ok(Vec::new());
    };
    match prompt {
        RequestPrompt::Inline(msgs) => Ok(msgs),
        RequestPrompt::Simple(text) => Ok(vec![Message::User(UserMessage {
            content: RichContent::Text(text),
            name: None,
        })]),
        RequestPrompt::File(path) => read_messages_file(path),
        RequestPrompt::PythonInline(code) => crate::python::exec_code(&code),
        RequestPrompt::PythonFile(path) => crate::python::exec_file(&path),
    }
}

pub(crate) fn read_messages_file(path: PathBuf) -> Result<Vec<Message>, Error> {
    let bytes = std::fs::read(&path)
        .map_err(|e| Error::PromptFileRead(path.clone(), e))?;
    let mut de = serde_json::Deserializer::from_slice(&bytes);
    serde_path_to_error::deserialize(&mut de)
        .map_err(Error::InlineDeserialize)
}

async fn resolve_agent(
    ctx: &Context,
    spec: AgentSpec,
) -> Result<InlineAgentBaseWithFallbacksOrRemoteCommitOptional, Error> {
    match spec {
        AgentSpec::Resolved(resolved) => Ok(resolved),
        AgentSpec::Favorite(name) => {
            let mut config = ctx.filesystem.read_config().await?;
            let favorites = config.agents().get_favorites();
            let fav = favorites
                .iter()
                .find(|f| f.get_name() == name)
                .ok_or_else(|| Error::FavoriteNotFound(name.clone()))?;
            Ok(InlineAgentBaseWithFallbacksOrRemoteCommitOptional::Remote(
                fav.path.clone(),
            ))
        }
    }
}

/// Compute the content-addressed `agent_full_id` (concatenated
/// base62 ids of the primary agent + each fallback) for `spec`
/// **before** the spawn fires. Used by the queue drain to address
/// PENDING tags that this spawn will bind on first chunk.
async fn resolve_agent_full_id(
    ctx: &Context,
    spec: &AgentSpec,
) -> Result<String, Error> {
    let path = match spec {
        AgentSpec::Resolved(InlineAgentBaseWithFallbacksOrRemoteCommitOptional::AgentBase(
            base,
        )) => {
            let with_ids = base
                .clone()
                .convert()
                .map_err(Error::AgentConvert)?;
            return Ok(with_ids.full_id());
        }
        AgentSpec::Resolved(InlineAgentBaseWithFallbacksOrRemoteCommitOptional::Remote(
            p,
        )) => p.clone(),
        AgentSpec::Favorite(name) => {
            let mut config = ctx.filesystem.read_config().await?;
            let favorites = config.agents().get_favorites();
            let fav = favorites
                .iter()
                .find(|f| f.get_name() == name)
                .ok_or_else(|| Error::FavoriteNotFound(name.clone()))?;
            fav.path.clone()
        }
    };
    let response = objectiveai_sdk::agent::get_agent(&ctx.http, path).await?;
    Ok(response.inner.full_id())
}

pub mod request_schema {
    use objectiveai_sdk::cli::command::agents::instances::spawn as sdk;
    use objectiveai_sdk::cli::command::agents::instances::spawn::request_schema::{Request, Response};

    use crate::context::Context;
    use crate::error::Error;

    pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
        Ok(objectiveai_sdk::cli::command::ResponseSchema(schemars::schema_for!(sdk::Request)))
    }
}

pub mod response_schema {
    use objectiveai_sdk::cli::command::agents::instances::spawn as sdk;
    use objectiveai_sdk::cli::command::agents::instances::spawn::response_schema::{Request, Response};

    use crate::context::Context;
    use crate::error::Error;

    pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
        Ok(objectiveai_sdk::cli::command::ResponseSchema(schemars::schema_for!(sdk::Response)))
    }
}
