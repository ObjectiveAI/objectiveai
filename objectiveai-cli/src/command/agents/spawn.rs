//! `agents spawn` — in-process chunk-or-id streaming handler.
//!
//! The agent input is the shared [`AgentSelector`] — a direct ref
//! (inline / file / python / remote), a tag, or an existing
//! instance. Tags resolve first: BOUND → the live hierarchy
//! (historic case), GROUPED → the group's stored spec plus the tag
//! threaded into the conduit for the BOUND upgrade, ABSENT → error.
//!
//! Stream-true (`dangerous_advanced.stream = Some(true)`): resolve
//! + lock + drive the SDK streaming WS connection inside this cli
//! process. The INITIAL lock (try_acquire, failure = error): historic
//! case → the AIH lock, un-upgraded tag case → the tag lock, plain ref
//! → no initial lock. When a parent `agents message` transferred a
//! claim into this process, the lockfile adopts it lazily on this first
//! `try_acquire`, so the acquisition succeeds instantly. Historic spawns
//! load their agent params +
//! continuation from the stored session. Mid-stream, every newly
//! revealed hierarchy gets a best-effort AIH claim
//! ([`AgentInstanceRegistry::observe`]); the first success releases
//! the tag claim. End-of-stream: if the hierarchy has undelivered
//! `message_queue` rows, restart with the latest continuation —
//! restart passes flow into the same output stream.
//!
//! Stream-false (the default): re-invoke `objectiveai-cli agents
//! spawn ...` as a **detached subprocess** with the same arguments
//! plus `stream = true` (so the resolution + locking above runs in
//! the child), read the first `ResponseItem::Id` line off the
//! child's stdout, yield it, and return. The subprocess runs
//! orphaned to completion (Unix: kernel re-parents to init;
//! Windows: `DETACHED_PROCESS | CREATE_NEW_PROCESS_GROUP` keeps
//! it alive past parent exit).
//!
//! `params.stream` on the wire is always `Some(true)`; the
//! `dangerous_advanced.stream` setting only controls cli-side
//! output.

use std::pin::Pin;

use futures::Stream;
use futures::StreamExt;
use objectiveai_sdk::agent::InlineAgentBaseWithFallbacksOrRemoteCommitOptional;
use objectiveai_sdk::agent::completions::message::{Message, UserMessage};
use objectiveai_sdk::agent::completions::request::AgentCompletionCreateParams;
use objectiveai_sdk::cli::command::agents::selector::{AgentRef, AgentSelector};
use objectiveai_sdk::cli::command::agents::spawn::{
    Request, RequestDangerousAdvanced, ResponseItem,
};
use objectiveai_sdk::cli::command::{BinaryExecutor, CommandExecutor};

use crate::context::Context;
use crate::error::Error;
use crate::websockets::agent_hierarchies::ChunkAgentHierarchies;
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

/// Stream-false: re-invoke `objectiveai-cli agents spawn`
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
            child_request.dangerous_advanced = Some(RequestDangerousAdvanced {
                stream: Some(true),
                ..Default::default()
            })
        }
    }
    // The child is a re-exec of this CLI — it must not inherit the
    // parent's transform / token budget (timeout survives).
    crate::command::reexec::strip_inherited(&mut child_request.base);

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

/// Spawn modes after selector resolution: a fresh agent (direct
/// ref, or a GROUPED tag carrying the tag name for the conduit
/// upgrade) or an existing hierarchy resumed via its stored
/// session + continuation.
enum Mode {
    Fresh {
        agent: InlineAgentBaseWithFallbacksOrRemoteCommitOptional,
        tag: Option<String>,
    },
    Historic {
        hierarchy: String,
    },
}

async fn execute_streaming(
    ctx: &Context,
    request: Request,
) -> Result<ItemStream, Error> {
    // Required user-message slot — gets wrapped into a single
    // `Message::User` at the head of the API call's `messages`
    // array. Reuses `agents message`'s `resolve_message`
    // so the five wire variants (`Simple` / `Inline(RichContent)`
    // / `File` / `PythonInline` / `PythonFile`) round-trip
    // identically. EMPTY resolved content (`--simple ""`, an empty
    // Inline text, empty parts) means a wake-up/resume turn: send an
    // EMPTY `messages` array — never a user message with an empty
    // string — and let the API drive from the continuation + the
    // conduit's queue drain.
    let content = super::message::resolve_message(ctx, request.message).await?;
    let messages = if content.is_empty() {
        Vec::new()
    } else {
        vec![Message::User(UserMessage {
            content,
        })]
    };
    let seed = request.dangerous_advanced.as_ref().and_then(|a| a.seed);

    // Resolve the agent target AND which laboratory targets its attachments
    // are keyed on. Labs only apply in Tag/Instance mode (a direct Ref has no
    // tag/AIH to key on). `run_multi_pass` does the actual DB resolution; here
    // we only name the targets — the same 3 permutations as before: a grouped
    // tag → the tag's labs; a resolved (Bound) tag → the tag's UNION the bound
    // AIH's; an instance → the AIH's.
    use crate::command::agents::locks::Family;
    use crate::db::laboratory_attachments::Target;
    let (mode, lab_targets, family): (Mode, Vec<Target>, Option<Family>) = match request.agent {
        AgentSelector::Ref { agent } => (
            Mode::Fresh {
                agent: resolve_agent_ref(ctx, agent).await?,
                tag: None,
            },
            Vec::new(),
            None,
        ),
        AgentSelector::Tag { agent_tag } => {
            match crate::db::tags::lookup(ctx.db_client().await?, &agent_tag).await? {
                crate::db::tags::LookupState::Bound { agent_instance_hierarchy } => {
                    let lab_targets = vec![
                        Target::Tag(agent_tag.clone()),
                        Target::Aih(agent_instance_hierarchy.clone()),
                    ];
                    (
                        Mode::Historic {
                            hierarchy: agent_instance_hierarchy.clone(),
                        },
                        lab_targets,
                        Some(Family::Hierarchy(agent_instance_hierarchy)),
                    )
                }
                crate::db::tags::LookupState::Grouped { agent_spec, tag_group_id, .. } => {
                    let lab_targets = vec![Target::Tag(agent_tag.clone())];
                    (
                        Mode::Fresh {
                            agent: agent_spec,
                            tag: Some(agent_tag),
                        },
                        lab_targets,
                        Some(Family::Group(tag_group_id)),
                    )
                }
                crate::db::tags::LookupState::Absent => {
                    return Err(Error::TagNotFound(agent_tag));
                }
            }
        }
        AgentSelector::Instance {
            parent_agent_instance_hierarchy,
            agent_instance,
        } => {
            let parent = parent_agent_instance_hierarchy
                .as_deref()
                .unwrap_or(&ctx.config.agent_instance_hierarchy);
            let hierarchy = format!("{parent}/{agent_instance}");
            let lab_targets = vec![Target::Aih(hierarchy.clone())];
            (
                Mode::Historic {
                    hierarchy: hierarchy.clone(),
                },
                lab_targets,
                Some(Family::Hierarchy(hierarchy)),
            )
        }
    };

    // Initial lock + params assembly. Acquire the agent's whole lock FAMILY
    // (all-or-nothing, NON-BLOCKING) so that while it's live none of its tags
    // can be relocated (`tags apply`) or have labs detached (`laboratories
    // detach`): a GROUPED tag locks every tag in its group; a bound tag / AIH
    // locks the AIH plus every tag bound to it. A held member means the agent
    // (or another spawn of the tag) is already live → error. When the parent
    // `agents message` transferred the family into this process, the lockfile
    // adopts each claim lazily on this first acquire, so they re-acquire
    // INSTANTLY rather than conflicting with the inherited handles. Mid-stream
    // best-effort AIH claims in `run_multi_pass` are unaffected.
    let state_dir = ctx.filesystem.state_dir();
    let mut registry = AgentInstanceRegistry::new(state_dir.clone(), ctx.agent_locks_arc());
    if let Some(family) = family {
        let is_group = matches!(family, Family::Group(_));
        match super::locks::try_acquire_family(
            ctx.agent_locks(),
            ctx.db_client().await?,
            &state_dir,
            family,
        )
        .await?
        {
            Some(fam) => {
                if let Some((hierarchy, aih_lock)) = fam.aih {
                    registry.preseed(hierarchy, aih_lock);
                }
                registry.hold_tag_claims(fam.tags);
            }
            None if is_group => {
                // GROUPED: name the requested tag in the error.
                let tag = match &mode {
                    Mode::Fresh { tag: Some(tag), .. } => tag.clone(),
                    _ => String::new(),
                };
                return Err(Error::AgentTagActive { tag });
            }
            None => {
                let agent_instance_hierarchy = match &mode {
                    Mode::Historic { hierarchy } => hierarchy.clone(),
                    _ => String::new(),
                };
                return Err(Error::AgentInstanceActive {
                    agent_instance_hierarchy,
                });
            }
        }
    }
    let (agent, agent_tag, continuation) = match mode {
        Mode::Fresh { agent, tag } => (agent, tag, None),
        Mode::Historic { hierarchy } => {
            let lookup = crate::db::logs::lookup_session(ctx.db_client().await?, &hierarchy)
                .await?
                .ok_or(Error::AgentNoPriorRequest {
                    agent_instance_hierarchy: hierarchy,
                })?;
            (lookup.agent, None, lookup.continuation)
        }
    };

    // Message-queue delivery to the live API happens through the
    // conduit's `read_pending_and_upgrade_tag` call — the API
    // pulls pending rows on demand as the stream runs and stamps
    // their ids onto the first emitted assistant chunk's
    // `request_message_ids`. No pre-spawn drain + prepend here.
    //
    // `run_multi_pass` builds the create-params and resolves the laboratory
    // attachments (from `lab_targets`) internally.
    let ctx_clone = ctx.clone();
    Ok(Box::pin(run_multi_pass(
        ctx_clone,
        messages,
        agent,
        seed,
        continuation,
        agent_tag,
        lab_targets,
        registry,
    )))
}

/// Drives one or more stream passes until no seen hierarchy has
/// pending `message_queue` items. Each pass opens a fresh WS
/// stream + log writer + MCP server + conduit; the
/// [`AgentInstanceRegistry`] (carrying any initial AIH/tag claim)
/// persists across passes so an agent's lock stays held for the
/// whole spawn lifetime, not per-pass — and is released when the
/// stream (and with it the registry) drops.
/// Resolve the laboratory ids attached to `lab_targets` into the request's
/// `laboratories` value. Lists every target CONCURRENTLY (one pool serves
/// concurrent queries), then flattens + dedups (first-seen order). `None` when
/// no targets / no attachments. Shared by `agents spawn` and `agents queue
/// deliver` (both go through `run_multi_pass`). No liveness check — the conduit
/// dials each laboratory on demand at MCP-initialize time.
async fn resolve_laboratories(
    ctx: &Context,
    lab_targets: &[crate::db::laboratory_attachments::Target],
) -> Result<Option<Vec<objectiveai_sdk::laboratories::Laboratory>>, Error> {
    if lab_targets.is_empty() {
        return Ok(None);
    }
    let pool = ctx.db_client().await?;
    let lists = futures::future::try_join_all(
        lab_targets
            .iter()
            .map(|target| crate::db::laboratory_attachments::list(pool, target)),
    )
    .await?;
    let mut ids: Vec<String> = Vec::new();
    for list in lists {
        for id in list {
            if !ids.contains(&id) {
                ids.push(id);
            }
        }
    }
    if ids.is_empty() {
        return Ok(None);
    }
    Ok(Some(
        ids.into_iter()
            .map(|id| {
                objectiveai_sdk::laboratories::Laboratory::Client(
                    objectiveai_sdk::laboratories::ClientLaboratory {
                        r#type: objectiveai_sdk::laboratories::ClientLaboratoryType::Client,
                        id,
                    },
                )
            })
            .collect(),
    ))
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn run_multi_pass(
    ctx: Context,
    messages: Vec<Message>,
    agent: InlineAgentBaseWithFallbacksOrRemoteCommitOptional,
    seed: Option<i64>,
    continuation: Option<String>,
    agent_tag: Option<String>,
    lab_targets: Vec<crate::db::laboratory_attachments::Target>,
    mut registry: AgentInstanceRegistry,
) -> impl Stream<Item = Result<ResponseItem, Error>> + Send {
    async_stream::try_stream! {
        // Resolve the agent's laboratory attachments (from the named targets)
        // and assemble the create-params. `provider`/`response_format` are
        // always defaulted and `stream` is always true for the in-process WS
        // path; only `messages`/`continuation` change across restart passes.
        let laboratories = resolve_laboratories(&ctx, &lab_targets).await?;
        let mut params = AgentCompletionCreateParams {
            messages,
            provider: None,
            agent,
            response_format: None,
            seed,
            stream: Some(true),
            continuation,
            laboratories,
        };
        // A spawn has exactly one `(agent_instance_hierarchy,
        // agent_full_id)` pair — set by the API on the very first
        // chunk and never changes across restart passes. Capture
        // once; reuse forever. `None` until the first chunk lands.
        let mut identity: Option<(String, String)> = None;
        // Has `ResponseItem::Id` been yielded yet? Persists across
        // restart passes — the spawn-id handshake is a one-time
        // event, gated on the LogWriter's `written_once` signal so
        // the caller only sees the Id after at least one log row
        // has been persisted.
        let mut id_emitted = false;
        // Resolve the MCP client tuning once for the whole spawn; every
        // pass's conduit reuses these (cheap to pass per pass).
        let mcp_timeout_ms = ctx.resolve_mcp_timeout_ms().await?;
        let backoff_max_elapsed_time_ms =
            ctx.resolve_backoff_max_elapsed_time_ms().await?;

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
                    mcp_timeout_ms,
                    backoff_max_elapsed_time_ms,
                );
            // Spawn.rs doesn't need the primary-id ready signal —
            // it yields `ResponseItem::Id` from
            // `chunk.agent_instance_hierarchy` directly on the first
            // chunk. Drop the receiver.
            let (log_writer, _ready_rx) = crate::db::logs::write_agent_completion(
                ctx.db_client().await?,
                &params,
                ctx.config.agent_instance_hierarchy.clone(),
            )
            .map_err(|e| Error::Instance(format!(
                "failed to build agent-completion log writer: {e}"
            )))?;

            let (sdk_stream, notifier) =
                objectiveai_sdk::agent::completions::create_agent_completion_streaming(
                    ctx.api_client().await?,
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
            // Per-pass buffer of chunks held back until the
            // LogWriter confirms it has persisted at least once.
            // Only meaningful for pass 1 — pass 2+ already has
            // `id_emitted = true` from a prior pass, so the buffer
            // gate never triggers and chunks flow through directly.
            let mut buffered: Vec<
                objectiveai_sdk::agent::completions::response::streaming::AgentCompletionChunk,
            > = Vec::new();
            let mut stream_err: Option<String> = None;

            while let Some(item) = sdk_stream.next().await {
                let chunk = match item {
                    Ok(c) => c,
                    Err(e) => {
                        stream_err = Some(format!("agent stream item error: {e}"));
                        break;
                    }
                };

                // First chunk EVER (first pass, first chunk):
                // capture the spawn's identity + claim the lock
                // file. Tag-group upgrade is owned by the conduit's
                // `read_pending_and_upgrade_tag`, which the API
                // fires before the very first chunk is produced —
                // no upgrade fan-out is needed here. The
                // `ResponseItem::Id` handshake itself fires later,
                // gated on `log_writer.written_once()`.
                if identity.is_none() {
                    let hier = chunk.agent_instance_hierarchy.clone();
                    let full_id = chunk.agent_full_id.clone();
                    registry.observe(&hier).await;
                    identity = Some((hier, full_id));
                }

                // Latest continuation seen on the wire — what we
                // use to restart if pending messages turn up at
                // EOF. Only the terminal chunk usually carries one.
                if let Some(c) = chunk.continuation.as_deref() {
                    last_continuation = Some(c.to_string());
                }

                // Upsert any `(AIH, continuation)` pairs the chunk
                // carries into the `agent_continuations` registry
                // (cumulative chunks always yield exactly one pair;
                // the Vec is 0-or-1 long depending on whether
                // `continuation` is `Some`). Awaited before the
                // log-writer send + downstream yield so the registry
                // row is visible by the time the chunk leaves this
                // body.
                let mut continuation_upserts: Vec<_> = Vec::new();
                for (hier, continuation) in chunk.agent_instance_hierarchies() {
                    if let Some(c) = continuation {
                        continuation_upserts.push(
                            crate::db::agent_continuations::upsert(ctx.db_client().await?, hier, c),
                        );
                    }
                }
                if let Err(e) =
                    futures::future::try_join_all(continuation_upserts).await
                {
                    stream_err =
                        Some(format!("agent_continuations upsert: {e}"));
                    break;
                }

                // Log + forward. The write is a synchronous mpsc
                // send into the LogWriter's listener task — DB IO
                // happens off this critical path. Clone the chunk
                // for the listener; the original yields downstream
                // (or sits in the buffer until the Id gate opens).
                if let Err(e) = log_writer.write(chunk.clone()) {
                    stream_err = Some(format!("log writer error: {e}"));
                    break;
                }

                // Id gate: once the LogWriter signals it has
                // persisted at least one batch, yield the Id and
                // drain any chunks buffered up to this point. The
                // gate flips exactly once per spawn (across all
                // passes) — `id_emitted` persists outside the
                // restart loop.
                if !id_emitted && log_writer.written_once() {
                    let (hier, _) = identity
                        .as_ref()
                        .expect("identity set above on the first chunk");
                    yield ResponseItem::Id(hier.clone());
                    for c in buffered.drain(..) {
                        yield ResponseItem::Chunk(c);
                    }
                    id_emitted = true;
                }

                if id_emitted {
                    yield ResponseItem::Chunk(chunk);
                } else {
                    buffered.push(chunk);
                }
            }

            // Post-stream: if the SDK closed before the LogWriter
            // ever flipped `written_once` true (e.g. very fast EOF
            // ahead of the listener's first batch), wait for the
            // first persistence to land, then emit the Id + drain
            // any held chunks. Only fires when we actually have
            // chunks queued behind the gate.
            if !id_emitted && !buffered.is_empty() {
                if let Err(e) = log_writer.wait_written_once().await {
                    stream_err.get_or_insert_with(|| format!("log writer wait: {e}"));
                } else {
                    let (hier, _) = identity
                        .as_ref()
                        .expect("identity set on the first chunk");
                    yield ResponseItem::Id(hier.clone());
                    for c in buffered.drain(..) {
                        yield ResponseItem::Chunk(c);
                    }
                    id_emitted = true;
                }
            }

            // Finalize the log writer (consumes it; drops the
            // sender; awaits the listener task). By construction
            // this returns only after the queue is empty AND no
            // work is in flight.
            if let Err(e) = log_writer.finalize().await {
                stream_err.get_or_insert_with(|| format!("log writer finalize: {e}"));
            }
            drop(sdk_stream);
            drop(conduit);

            if let Some(e) = stream_err {
                Err(Error::Instance(e))?;
            }

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
                ctx.db_client().await?, hier,
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

/// Resolve an [`AgentRef`] into a typed agent. `Resolved` passes
/// through; `File` / `PythonInline` / `PythonFile` run their IO /
/// Python here via the shared 5-variant resolver (the `simple`
/// slot is never populated for agent refs — `--agent <ref>`
/// strings parse at the clap layer).
pub(crate) async fn resolve_agent_ref(
    ctx: &Context,
    agent: AgentRef,
) -> Result<InlineAgentBaseWithFallbacksOrRemoteCommitOptional, Error> {
    let (file, python_inline, python_file) = match agent {
        AgentRef::Resolved(resolved) => return Ok(resolved),
        AgentRef::File(p) => (Some(p), None, None),
        AgentRef::PythonInline(code) => (None, Some(code), None),
        AgentRef::PythonFile(p) => (None, None, Some(p)),
    };
    crate::source_resolver::resolve_source(
        ctx,
        None,
        None,
        file,
        python_inline,
        python_file,
        |_| unreachable!("agent refs have no plain-text variant"),
    )
    .await
}

pub mod request_schema {
    use objectiveai_sdk::cli::command::agents::spawn as sdk;
    use objectiveai_sdk::cli::command::agents::spawn::request_schema::{Request, Response};

    use crate::context::Context;
    use crate::error::Error;

    pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
        Ok(objectiveai_sdk::cli::command::ResponseSchema(schemars::schema_for!(sdk::Request)))
    }
}

pub mod response_schema {
    use objectiveai_sdk::cli::command::agents::spawn as sdk;
    use objectiveai_sdk::cli::command::agents::spawn::response_schema::{Request, Response};

    use crate::context::Context;
    use crate::error::Error;

    pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
        Ok(objectiveai_sdk::cli::command::ResponseSchema(schemars::schema_for!(sdk::Response)))
    }
}
