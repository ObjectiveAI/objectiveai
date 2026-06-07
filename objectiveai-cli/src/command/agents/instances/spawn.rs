//! `agents spawn` — bare-naked chunk-or-id streaming handler.
//!
//! Spawns the instance runner as a detached background process. The
//! streamed item shape depends on `dangerous_advanced.stream`:
//!
//! - **`None | Some(false)` (default)** — yields exactly one
//!   `ResponseItem::Id(<agent_instance_hierarchy>)` once the runner's
//!   `LogStreamReady` handshake fires, then the leaf's stream ends.
//!   The instance runner child keeps running orphaned and drives the
//!   completion to completion.
//! - **`Some(true)`** — yields the same `ResponseItem::Id` first, then
//!   one `ResponseItem::Chunk` per chunk Notification the runner
//!   emits, until the runner's stdout EOFs.
//!
//! `params.stream` on the wire is always `Some(true)` regardless of the
//! `dangerous_advanced.stream` field — the latter controls the leaf's
//! output stream behaviour, not the API request.

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
    AgentSpec, Request, RequestPrompt, ResponseItem,
};

use crate::context::Context;
use crate::db;
use crate::error::Error;
use crate::streaming::{InstanceItem, instance_subprocess_stream};

type ItemStream = Pin<Box<dyn Stream<Item = Result<ResponseItem, Error>> + Send>>;

pub async fn execute(ctx: &Context, request: Request) -> Result<ItemStream, Error> {
    let mut messages = resolve_prompt(request.prompt)?;

    // Drain the queue *before* the instance fires. Two-rule
    // predicate (see `db::prompts::drain_for_spawn_async`):
    //   1. queue items addressed to `request.agent_tag` directly
    //   2. queue items addressed to any PENDING tag whose
    //      (parent, agent_full_id) matches this spawn — i.e. tags
    //      that the first-chunk auto-promotion hook will bind.
    // The drained items get joined oldest-first with `\n\n`
    // separators and prepended as a fresh User message at the head
    // of `messages`.
    let agent_full_id = resolve_agent_full_id(ctx, &request.agent).await?;
    let drained = db::prompts::drain_for_spawn(
        &ctx.db,
        &ctx.config.agent_instance_hierarchy,
        &agent_full_id,
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

    let params = AgentCompletionCreateParams {
        messages,
        provider: None,
        agent,
        response_format: None,
        seed: request.seed,
        stream: Some(true),
        continuation: None,
    };

    let stream = request
        .dangerous_advanced
        .as_ref()
        .and_then(|a| a.stream)
        .unwrap_or(false);

    let raw = instance_subprocess_stream(
        ctx,
        crate::instance::request::InstanceEndpoint::AgentsSpawn(params),
        request.agent_tag,
        stream,
    );
    // Peek the first item before returning the stream. If the
    // head is Err (or the producer closes without yielding
    // anything), restore the drained queue rows via
    // `db::prompts::re_enqueue_async` and surface the original
    // error. On Ok, hand back a `StreamOnce::new(head).chain(tail)`
    // — same pattern objectiveai-api's
    // `functions/executions/client.rs::create_streaming_handle_usage`
    // uses for peek-then-stream returns.
    let mut tail: ItemStream = Box::pin(raw.map(map_item));
    match tail.as_mut().next().await {
        Some(Ok(first)) => Ok(Box::pin(
            objectiveai_sdk::cli::command::StreamOnce::new(Ok(first)).chain(tail),
        )),
        Some(Err(e)) => {
            let r = db::prompts::re_enqueue(&ctx.db, drained).await;
            Err(crate::command::message_queue_drain::combine_drain_failure(e, r))
        }
        None => {
            let r = db::prompts::re_enqueue(&ctx.db, drained).await;
            Err(crate::command::message_queue_drain::combine_drain_failure(
                Error::EmptyStream,
                r,
            ))
        }
    }
}

fn map_item(item: Result<InstanceItem, Error>) -> Result<ResponseItem, Error> {
    match item? {
        InstanceItem::Id(id) => Ok(ResponseItem::Id(id)),
        InstanceItem::Chunk(value) => serde_json::from_value(value)
            .map(ResponseItem::Chunk)
            .map_err(|e| Error::InlineJson(e)),
    }
}

pub(crate) fn resolve_prompt(prompt: RequestPrompt) -> Result<Vec<Message>, Error> {
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
///
/// - `AgentSpec::Resolved(InlineAgentBaseWithFallbacks)`: compute
///   locally via `.convert().full_id()` — no HTTP.
/// - `AgentSpec::Resolved(Remote)` and `AgentSpec::Favorite`: fetch
///   the remote definition via the same `objectiveai_sdk::agent::
///   get_agent` call `agents get` uses, then call `.full_id()` on
///   the returned `RemoteAgentWithFallbacks`.
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
