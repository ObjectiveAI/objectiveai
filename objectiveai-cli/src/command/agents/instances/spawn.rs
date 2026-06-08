//! `agents spawn` — in-process chunk-or-id streaming handler.
//!
//! Stream-true (`dangerous_advanced.stream = Some(true)`): drive
//! the SDK streaming WS connection directly inside this cli
//! process. Per-chunk: claim a process-owned file for every new
//! `agent_instance_hierarchy` and upgrade any PENDING tags that
//! map to it. End-of-stream: check whether any seen hierarchy has
//! undelivered `message_queue` rows; destroy the claim files of
//! the ones that don't (free their slot); if any do, restart with
//! the same params minus messages plus the latest captured
//! continuation. Yields chunks straight through as they arrive —
//! restart passes flow into the same output stream.
//!
//! Stream-false (`dangerous_advanced.stream = None | Some(false)`,
//! the default): re-invoke `objectiveai-cli agents instances spawn
//! ...` as a **detached subprocess** with the same arguments plus
//! `stream = true`, read the first `ResponseItem::Id` line off the
//! child's stdout, yield it, and return. The subprocess runs
//! orphaned to completion (Unix: kernel re-parents to init;
//! Windows: `DETACHED_PROCESS | CREATE_NEW_PROCESS_GROUP` keeps
//! it alive past parent exit). This is how `agents spawn` has
//! always behaved externally — only the implementation moved from
//! the dedicated `instance` subcommand to a self-respawn of the
//! ordinary cli command.
//!
//! `params.stream` on the wire is always `Some(true)`; the
//! `dangerous_advanced.stream` setting only controls cli-side
//! output.

use std::path::PathBuf;
use std::pin::Pin;

use futures::Stream;
use futures::StreamExt;
use objectiveai_sdk::agent::InlineAgentBaseWithFallbacksOrRemoteCommitOptional;
use objectiveai_sdk::agent::completions::message::{Message, UserMessage};
use objectiveai_sdk::agent::completions::request::AgentCompletionCreateParams;
use objectiveai_sdk::cli::command::agents::instances::spawn::{
    AgentResolution, AgentSpec, Request, RequestDangerousAdvanced, ResponseItem,
};
use objectiveai_sdk::cli::command::{BinaryExecutor, CommandExecutor};

use crate::context::Context;
use crate::error::Error;
use crate::websockets::agent_registry::AgentInstanceRegistry;

type ItemStream = Pin<Box<dyn Stream<Item = Result<ResponseItem, Error>> + Send>>;

pub async fn execute(
    ctx: &Context,
    request: Request,
) -> Result<ItemStream, Error> {
    let want_stream = request
        .dangerous_advanced
        .as_ref()
        .and_then(|a| a.stream)
        .unwrap_or(false);
    if want_stream {
        execute_streaming(ctx, request).await
    } else {
        execute_detached(request).await
    }
}

/// Stream-false: re-invoke `objectiveai-cli agents instances spawn`
/// as a detached subprocess with `stream = true`, capture the
/// first `ResponseItem::Id` off the child's stdout, yield it, and
/// return. The subprocess outlives this call — its
/// `tokio::process::Child` handle is dropped without kill (the
/// SDK's `BinaryExecutor` default + Windows `DETACHED_PROCESS`
/// flag).
async fn execute_detached(request: Request) -> Result<ItemStream, Error> {
    // Re-invoke with stream=true so the child runs the real
    // streaming path. Same argv otherwise — `BinaryExecutor` will
    // ask `Request::into_command()` for it.
    let mut child_request = request;
    match child_request.dangerous_advanced.as_mut() {
        Some(adv) => adv.stream = Some(true),
        None => {
            child_request.dangerous_advanced =
                Some(RequestDangerousAdvanced { stream: Some(true) })
        }
    }

    // Self-respawn: point the executor at *this* binary (whichever
    // path the OS recorded for the current process), then arm
    // Windows-detach so the child survives parent exit. Unix gets
    // re-parent-to-init for free via the default kill_on_drop=false.
    let exe = std::env::current_exe()
        .map_err(|e| Error::Spawn("current_exe".into(), e))?;
    let executor = BinaryExecutor::from_path(exe).detach(true);

    let mut stream = executor
        .execute::<Request, ResponseItem>(child_request, None)
        .await
        .map_err(|e| Error::Instance(format!(
            "self-respawn for agents spawn: {e}"
        )))?;

    // Take exactly the first ResponseItem (the LogStreamReady Id),
    // yield it, return. Drop the rest of the stream + the Child
    // handle without kill. On Windows the detach flags keep the
    // child running; on Unix the kernel re-parents to init.
    let first = stream
        .next()
        .await
        .ok_or(Error::EmptyStream)?
        .map_err(|e| Error::Instance(format!(
            "self-respawn for agents spawn: {e}"
        )))?;
    Ok(Box::pin(
        objectiveai_sdk::cli::command::StreamOnce::new(Ok(first)),
    ))
}

async fn execute_streaming(
    ctx: &Context,
    request: Request,
) -> Result<ItemStream, Error> {
    // Required user-message slot — gets wrapped into a single
    // `Message::User` at the head of the API call's `messages`
    // array. Reuses `agents instances message`'s `resolve_message`
    // so the five wire variants (`Simple` / `Inline(RichContent)`
    // / `File` / `PythonInline` / `PythonFile`) round-trip
    // identically.
    let content = super::message::resolve_message(request.message)?;
    let messages = vec![Message::User(UserMessage {
        content,
        name: None,
    })];
    // Dispatch on the resolution mode:
    // - Direct → use the `AgentSpec` as-is (favorite resolution runs
    //   in `resolve_agent`).
    // - Tag    → look the tag up: BOUND is rejected (the AIH is
    //   already live; `agents instances message` is the right entry
    //   point). Grouped takes the group's stored AgentSpec.
    let (agent_spec, agent_tag) = match request.agent {
        AgentResolution::Direct { agent_spec } => (agent_spec, None),
        AgentResolution::Tag { agent_tag } => {
            match crate::db::tags::lookup(&ctx.db, &agent_tag).await? {
                crate::db::tags::LookupState::Bound { agent_instance_hierarchy } => {
                    return Err(Error::Instance(format!(
                        "tag {agent_tag:?} is already bound to {agent_instance_hierarchy:?}; \
                         use `agents instances message` to deliver to the live spawn"
                    )));
                }
                crate::db::tags::LookupState::Grouped {
                    agent_spec,
                    ..
                } => (agent_spec, Some(agent_tag)),
                crate::db::tags::LookupState::Absent => {
                    return Err(Error::TagNotFound(agent_tag));
                }
            }
        }
    };
    let agent = resolve_agent(ctx, agent_spec).await?;
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

    // Message-queue delivery to the live API happens through the
    // conduit's `read_pending_and_upgrade_tag` / `clear_by_ids`
    // calls — the API pulls pending rows on demand as the stream
    // runs. No pre-spawn drain + prepend here.
    let ctx_clone = ctx.clone();
    Ok(Box::pin(run_multi_pass(ctx_clone, params, agent_tag, agents_dir)))
}

/// Drives one or more stream passes until no seen hierarchy has
/// pending `message_queue` items. Each pass opens a fresh WS
/// stream + log writer + MCP server + conduit; the
/// [`AgentInstanceRegistry`] persists across passes so an agent's
/// process-owned claim file stays held for the whole spawn
/// lifetime, not per-pass.
pub(crate) fn run_multi_pass(
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
        // A spawn has exactly one `(agent_instance_hierarchy,
        // agent_full_id)` pair — set by the API on the very first
        // chunk and never changes across restart passes. Capture
        // once; reuse forever. `None` until the first chunk lands.
        let mut identity: Option<(String, String)> = None;

        loop {
            // Per-pass resources. New WS connection, new log writer,
            // new conduit + MCP server. The registry survives across
            // passes (see above).
            let mcp_server =
                crate::websockets::mcp_server::spawn(ctx.clone());
            let conduit =
                crate::websockets::conduit::ConduitMcpHandler::new(
                    mcp_server,
                    ctx.clone(),
                    agent_tag.clone(),
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
            let mut last_continuation: Option<String> = None;

            while let Some(item) = sdk_stream.next().await {
                let chunk = item.map_err(|e| {
                    Error::Instance(format!("agent stream item error: {e}"))
                })?;

                // First chunk EVER (first pass, first chunk):
                // capture the spawn's identity, claim the lock
                // file, and emit the `ResponseItem::Id` handshake.
                // Tag-group upgrade is owned by the conduit's
                // `read_pending_and_upgrade_tag`, which the API
                // fires before the very first chunk is produced —
                // no upgrade fan-out is needed here.
                if identity.is_none() {
                    let hier = chunk.agent_instance_hierarchy.clone();
                    let full_id = chunk.agent_full_id.clone();
                    registry.observe(&hier);
                    yield ResponseItem::Id(hier.clone());
                    identity = Some((hier, full_id));
                }

                // Latest continuation seen on the wire — what we
                // use to restart if pending messages turn up at
                // EOF. Only the terminal chunk usually carries one.
                if let Some(c) = chunk.continuation.as_deref() {
                    last_continuation = Some(c.to_string());
                }

                // Log + forward. No per-chunk registry / upgrade /
                // pending probe — the hierarchy is invariant and
                // those side effects already fired on the first
                // chunk above.
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

            // End-of-pass: a pure EXISTS check against the spawn's
            // single hierarchy. The conduit already promoted every
            // sibling tag in the group during its in-stream reads
            // via `read_pending_and_upgrade_tag` — so this check
            // sees the post-upgrade `tags` state and catches
            // anything queued mid-stream against a now-BOUND
            // sibling. On `false`, fall through to the implicit
            // registry drop on function return (no explicit destroy
            // needed — there's only one claim and we're done with it).
            let Some((hier, _full_id)) = identity.as_ref() else {
                // Empty stream — nothing was claimed, nothing to
                // restart. Just exit.
                break;
            };
            let pending = crate::db::message_queue::check_any_pending(
                &ctx.db, hier,
            )
            .await
            .unwrap_or(false);
            if !pending {
                break;
            }

            // Restart with the latest continuation only. No new
            // messages — the API picks up state from the
            // continuation token.
            params.messages = Vec::new();
            params.continuation = last_continuation;
        }
    }
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
