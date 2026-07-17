//! `agents spawn` — in-process chunk-or-id streaming handler.
//!
//! The agent input is the shared [`AgentSelector`] — a direct ref
//! (inline / file / python / remote), a tag, or an existing
//! instance. Tags resolve first: BOUND → the live hierarchy
//! (historic case), GROUPED → the group's stored spec plus the tag
//! threaded into the conduit for the BOUND upgrade, ABSENT → error.
//!
//! Stream-true (`dangerous_advanced.stream = Some(true)`): resolve
//! + lock + drive the SDK streaming WS connection inside this daemon
//! process. The INITIAL lock (try_acquire, failure = error): historic
//! case → the AIH lock, un-upgraded tag case → the tag lock, plain ref
//! → no initial lock. Agent locks are the daemon's in-process
//! [`AgentLockMap`](super::locks::AgentLockMap) — nothing on disk, no
//! cross-process claim or transfer. Historic spawns
//! load their agent params +
//! continuation from the stored session. Mid-stream, every newly
//! revealed hierarchy gets a best-effort AIH claim
//! ([`AgentInstanceRegistry::observe`]); the first success releases
//! the tag claim. End-of-stream: if the hierarchy has undelivered
//! `message_queue` rows, restart with the latest continuation —
//! restart passes flow into the same output stream.
//!
//! Stream-false (the default): run the same real streaming path
//! (`stream = true`, so the resolution + locking above run in the
//! task) as a **detached in-process daemon task**
//! ([`crate::command::detached::spawn_detached`]), yield the first
//! `ResponseItem::Id` it produces, and return. The task outlives this
//! call and drains to completion on the daemon's runtime; its lock
//! family releases when its stream ends.
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
use crate::context::{GlobalContext, ScopedContext};
use crate::error::Error;
use crate::http::agent_hierarchies::ChunkAgentHierarchies;
use crate::http::agent_registry::AgentInstanceRegistry;

type ItemStream = Pin<Box<dyn Stream<Item = Result<ResponseItem, Error>> + Send>>;

pub async fn execute(
    global: &GlobalContext, scoped: &ScopedContext,
    request: Request,
) -> Result<ItemStream, Error> {
    let want_stream = request
        .dangerous_advanced
        .as_ref()
        .and_then(|a| a.stream)
        .unwrap_or(false);
    if want_stream {
        execute_streaming(global, scoped, request).await
    } else {
        execute_detached(global, scoped, request).await
    }
}

/// Stream-false: run the real streaming path (`stream = true`) as a
/// detached in-process daemon task and surface only its first item —
/// the gated `Id`. The task outlives this call and drains to
/// completion (see [`crate::command::detached::spawn_detached`]); the
/// agent's lock family releases when its stream ends.
async fn execute_detached(global: &GlobalContext, scoped: &ScopedContext, request: Request) -> Result<ItemStream, Error> {
    // Re-invoke with stream=true so the detached run takes the real
    // streaming path (resolution + locking above run in the task).
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
    // The child re-enters via `crate::run` — it must not re-apply the
    // parent's transform / token budget (timeout survives).
    crate::command::reexec::strip_inherited(&mut child_request.base);

    Ok(crate::command::detached::spawn_detached::<Request, ResponseItem>(
        global.clone(),
        scoped.clone(),
        child_request,
        |_| Some(true),
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
    global: &GlobalContext, scoped: &ScopedContext,
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
    let content = super::message::resolve_message(global, scoped, request.message).await?;
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
    // Spawn-by-SPEC detection for the agent_refs registry: only a
    // direct `--agent` ref counts — spawns via tag or AIH resume or
    // rebind an existing agent and record nothing.
    let by_spec = matches!(request.agent, AgentSelector::Ref { .. });
    let (mode, lab_targets, family): (Mode, Vec<Target>, Option<Family>) = match request.agent {
        AgentSelector::Ref { agent } => (
            Mode::Fresh {
                agent: resolve_agent_ref(global, scoped, agent).await?,
                tag: None,
            },
            Vec::new(),
            None,
        ),
        AgentSelector::Tag { agent_tag } => {
            match crate::db::tags::lookup(&global.db_client().await?, &agent_tag).await? {
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
                .unwrap_or(scoped.agent_instance_hierarchy());
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
    // can be relocated (`tags apply`): a GROUPED tag locks every tag in its
    // group; a bound tag / AIH locks the AIH plus every tag bound to it.
    // (Laboratory attach/detach is deliberately NOT lock-guarded — it works
    // at any time; each restart pass re-resolves and dials whatever is
    // attached at that moment.) A held member means the agent
    // (or another spawn of the tag) is already live → error. When the parent
    // `agents message` transferred the family into this process, the lockfile
    // adopts each claim lazily on this first acquire, so they re-acquire
    // INSTANTLY rather than conflicting with the inherited handles. Mid-stream
    // best-effort AIH claims in `run_multi_pass` are unaffected.
    let state_dir = scoped.filesystem.state_dir();
    let mut registry = AgentInstanceRegistry::new(
        state_dir.clone(),
        global.agent_locks_arc(),
        global.resident_hubs().map(|h| h.active.clone()),
    );
    if let Some(family) = family {
        let is_group = matches!(family, Family::Group(_));
        match super::locks::try_acquire_family(
            global.agent_locks(),
            &global.db_client().await?,
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
            // The AIH lock is HELD here (family acquired above) —
            // failures are loggable, per the rule. Ad-hoc tee: the
            // spawn-long tee is created inside `run_multi_pass`,
            // which this error prevents from ever starting.
            let lookup = async {
                crate::db::logs::lookup_session(&global.db_client().await?, &hierarchy)
                    .await?
                    .ok_or(Error::AgentNoPriorRequest {
                        agent_instance_hierarchy: hierarchy.clone(),
                    })
            }
            .await;
            let lookup = match lookup {
                Ok(lookup) => lookup,
                Err(e) => {
                    let tee =
                        crate::db::logs::ConversationTee::spawn(global.resident_hubs().map(|h| h.conversations.clone()));
                    note_error(global, scoped, &tee, Some(&hierarchy), None, &e).await;
                    return Err(e);
                }
            };
            (lookup.agent, None, lookup.continuation)
        }
    };

    // The definition source to record at AIH-lock acquisition —
    // spawn-by-SPEC only.
    let agent_ref = if by_spec {
        match &agent {
            InlineAgentBaseWithFallbacksOrRemoteCommitOptional::Remote(remote) => {
                crate::db::agent_refs::AgentRefValue::remote(remote)
            }
            InlineAgentBaseWithFallbacksOrRemoteCommitOptional::AgentBase(spec) => {
                crate::db::agent_refs::AgentRefValue::inline(spec)
            }
        }
    } else {
        None
    };

    // Message-queue delivery to the live API happens through the
    // conduit's `read_pending_and_upgrade_tag` call — the API
    // pulls pending rows on demand as the stream runs and stamps
    // their ids onto the first emitted assistant chunk's
    // `request_message_ids`. No pre-spawn drain + prepend here.
    //
    // `run_multi_pass` builds the create-params and resolves the laboratory
    // attachments (from `lab_targets`) internally.
    Ok(Box::pin(run_multi_pass(
        global.clone(),
        scoped.clone(),
        messages,
        agent,
        seed,
        continuation,
        agent_tag,
        lab_targets,
        agent_ref,
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
    global: &GlobalContext, _scoped: &ScopedContext,
    lab_targets: &[crate::db::laboratory_attachments::Target],
) -> Result<Option<Vec<objectiveai_sdk::laboratories::Laboratory>>, Error> {
    if lab_targets.is_empty() {
        return Ok(None);
    }
    let pool = global.db_client().await?;
    let lists = futures::future::try_join_all(
        lab_targets
            .iter()
            .map(|target| crate::db::laboratory_attachments::list(&pool, target)),
    )
    .await?;
    // Dedup by BARE id, earliest attachment wins: an agent dials at
    // most one laboratory per id (the session's `ws://laboratory/{id}`
    // upstreams key by id). Each kept record's (machine, machine_state)
    // pair rides the ClientLaboratory so downstream routing is exact.
    let mut records: Vec<crate::db::laboratory_attachments::AttachmentRecord> = Vec::new();
    for list in lists {
        for record in list {
            if !records.iter().any(|r| r.laboratory_id == record.laboratory_id) {
                records.push(record);
            }
        }
    }
    if records.is_empty() {
        return Ok(None);
    }
    Ok(Some(
        records
            .into_iter()
            .map(|record| {
                objectiveai_sdk::laboratories::Laboratory::Client(
                    objectiveai_sdk::laboratories::ClientLaboratory {
                        r#type: objectiveai_sdk::laboratories::ClientLaboratoryType::Client,
                        id: record.laboratory_id,
                        machine: record.machine_id,
                        machine_state: record.machine_state,
                    },
                )
            })
            .collect(),
    ))
}

/// Record the ACTIVE laboratory set — the ids this pass is about to
/// send — under the AIH (most-recent-value semantics; empty when the
/// pass sends none). No-op while the AIH is still unknown (first-ever
/// spawn before the first chunk): the identity-minting site re-records
/// as soon as the AIH exists.
async fn record_active_laboratories(
    global: &GlobalContext,
    aih: Option<&str>,
    laboratories: &Option<Vec<objectiveai_sdk::laboratories::Laboratory>>,
) -> Result<(), Error> {
    let Some(aih) = aih else {
        return Ok(());
    };
    let ids: Vec<String> = laboratories
        .as_deref()
        .unwrap_or(&[])
        .iter()
        .map(|lab| match lab {
            objectiveai_sdk::laboratories::Laboratory::Client(client) => client.id.clone(),
            // Attachment-derived sets only ever carry Client markers,
            // but the active-set record keys by raw id either way.
            objectiveai_sdk::laboratories::Laboratory::Agent(agent) => agent.id.clone(),
        })
        .collect();
    let pool = global.db_client().await?;
    crate::db::agent_active_laboratories::replace(&pool, aih, &ids).await?;
    Ok(())
}

/// Persist + tee one spawn-path error. THE LOGGING RULE: an error is
/// recorded iff the AIH is known at the moment it occurs — the AIH
/// lock is held (Historic / Instance spawns hold it from acquisition;
/// `observe` claims it on the first chunk) or identity has been
/// minted. `aih == None` (a grouped-tag / ref spawn failing before its
/// first chunk) is a silent no-op by design. Best-effort: its own
/// failure is swallowed — there is nowhere left to report it.
pub(crate) async fn note_error(
    global: &GlobalContext, _scoped: &ScopedContext,
    tee: &crate::db::logs::ConversationTee,
    aih: Option<&str>,
    response_id: Option<&str>,
    error: &Error,
) {
    let Some(aih) = aih else { return };
    let Ok(pool) = global.db_client().await else {
        return;
    };
    let value = error.output_message();
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    // Persist BEFORE returning the error to the caller; tee after (the
    // tee is fire-and-forget).
    if crate::db::logs::insert_error(&pool, aih, response_id, &value, timestamp)
        .await
        .is_ok()
    {
        tee.send(crate::db::logs::error_frame(
            aih.to_string(),
            response_id.map(str::to_string),
            value,
            timestamp,
        ));
    }
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn run_multi_pass(
    global: GlobalContext,
    scoped: ScopedContext,
    messages: Vec<Message>,
    agent: InlineAgentBaseWithFallbacksOrRemoteCommitOptional,
    seed: Option<i64>,
    continuation: Option<String>,
    agent_tag: Option<String>,
    lab_targets: Vec<crate::db::laboratory_attachments::Target>,
    agent_ref: Option<crate::db::agent_refs::AgentRefValue>,
    mut registry: AgentInstanceRegistry,
) -> impl Stream<Item = Result<ResponseItem, Error>> + Send {
    async_stream::try_stream! {
        // Borrow the owned pair once — every use below is by-ref, and
        // per-pass resource constructors clone off these refs.
        let (global, scoped) = (&global, &scoped);
        let mut agent_ref = agent_ref;
        // One live-conversation tee for the whole spawn (created FIRST
        // so even pre-loop failures can ship their error frame): every
        // pass's log writer shares the one daemon socket connection.
        let conversation_tee =
            crate::db::logs::ConversationTee::spawn(global.resident_hubs().map(|h| h.conversations.clone()));
        // Resolve the agent's laboratory attachments (from the named targets)
        // and assemble the create-params. `provider`/`response_format` are
        // always defaulted and `stream` is always true for the in-process WS
        // path; only `messages`/`continuation` change across restart passes.
        let laboratories = match resolve_laboratories(global, scoped, &lab_targets).await {
            Ok(laboratories) => laboratories,
            Err(e) => {
                note_error(global, scoped, &conversation_tee, registry.aih(), None, &e).await;
                Err(e)?;
                unreachable!("Err(e)? diverges");
            }
        };
        // Record the ACTIVE set this pass will send (no-op while the
        // AIH is unknown — the identity-minting site covers that).
        if let Err(e) =
            record_active_laboratories(global, registry.aih(), &laboratories).await
        {
            note_error(global, scoped, &conversation_tee, registry.aih(), None, &e).await;
            Err(e)?;
            unreachable!("Err(e)? diverges");
        }
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
        // Resolve the MCP retry budget once for the whole spawn; every
        // pass's conduit reuses it (cheap to pass per pass). No MCP
        // timeout — the daemon never bounds its own MCP calls.
        let backoff_max_elapsed_time_ms =
            match crate::context::resolve_backoff_max_elapsed_time_ms(&scoped.filesystem).await {
                Ok(v) => v,
                Err(e) => {
                    note_error(global, scoped, &conversation_tee, registry.aih(), None, &e)
                        .await;
                    Err(e)?;
                    unreachable!("Err(e)? diverges");
                }
            };
        // Track the most recent response id seen on the wire — the id
        // logged errors attach to once a stream has existed.
        let mut last_response_id: Option<String> = None;

        loop {
            // Per-pass resources. New WS connection, new log writer,
            // new conduit + MCP server. The registry survives across
            // passes (see above).
            let mcp_server =
                crate::http::mcp_server::spawn(global.clone(), scoped.clone());
            let conduit =
                crate::http::conduit::ConduitMcpHandler::new(
                    mcp_server,
                    global.clone(),
                    scoped.clone(),
                    agent_tag.clone(),
                    backoff_max_elapsed_time_ms,
                );
            // Spawn.rs doesn't need the primary-id ready signal —
            // it yields `ResponseItem::Id` from
            // `chunk.agent_instance_hierarchy` directly on the first
            // chunk. Drop the receiver.
            let pool = match global.db_client().await {
                Ok(pool) => pool,
                Err(e) => {
                    let e = Error::from(e);
                    note_error(
                        global,
                        scoped,
                        &conversation_tee,
                        identity.as_ref().map(|(h, _)| h.as_str()).or(registry.aih()),
                        last_response_id.as_deref(),
                        &e,
                    )
                    .await;
                    Err(e)?;
                    unreachable!("Err(e)? diverges");
                }
            };
            let (log_writer, _ready_rx) = match crate::db::logs::write_agent_completion(
                &pool,
                &params,
                scoped.agent_instance_hierarchy().to_string(),
                Some(conversation_tee.clone()),
            )
            .map_err(|e| Error::Instance(format!(
                "failed to build agent-completion log writer: {e}"
            ))) {
                Ok(writer) => writer,
                Err(e) => {
                    note_error(
                        global,
                        scoped,
                        &conversation_tee,
                        identity.as_ref().map(|(h, _)| h.as_str()).or(registry.aih()),
                        last_response_id.as_deref(),
                        &e,
                    )
                    .await;
                    Err(e)?;
                    unreachable!("Err(e)? diverges");
                }
            };

            let stream_open = async {
                objectiveai_sdk::agent::completions::create_agent_completion_streaming(
                    scoped.api_client(global).await?,
                    params.clone(),
                    conduit.clone(),
                )
                .await
                .map_err(|e| Error::Instance(format!(
                    "failed to open agent-completion stream: {e}"
                )))
            }
            .await;
            let (sdk_stream, notifier) = match stream_open {
                Ok(opened) => opened,
                Err(e) => {
                    note_error(
                        global,
                        scoped,
                        &conversation_tee,
                        identity.as_ref().map(|(h, _)| h.as_str()).or(registry.aih()),
                        last_response_id.as_deref(),
                        &e,
                    )
                    .await;
                    Err(e)?;
                    unreachable!("Err(e)? diverges");
                }
            };
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

                // The id every subsequently-logged error attaches to.
                last_response_id = Some(chunk.id.clone());

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
                    // Spawn-by-spec: record the definition source the
                    // moment the AIH lock is acquired.
                    if let Some(value) = agent_ref.take() {
                        let upsert = async {
                            crate::db::agent_refs::upsert(
                                &global.db_client().await?,
                                &hier,
                                value,
                            )
                            .await?;
                            Ok::<_, Error>(())
                        }
                        .await;
                        if let Err(e) = upsert {
                            // Fold into the consolidated raise: it
                            // runs AFTER `log_writer.finalize()`, so
                            // the error row lands after every queued
                            // conversation row — never out of order.
                            stream_err = Some(format!("agent_refs upsert: {e}"));
                            break;
                        }
                    }
                    // First-ever spawns learn their AIH here — record
                    // the ACTIVE set the initial resolve couldn't
                    // (it had no AIH yet). Folds into the consolidated
                    // raise like the agent_refs upsert above.
                    if let Err(e) =
                        record_active_laboratories(global, Some(&hier), &params.laboratories)
                            .await
                    {
                        stream_err = Some(format!("active laboratories record: {e}"));
                        break;
                    }
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
                // Declared BEFORE the futures Vec so it outlives the
                // borrows the upsert futures hold (drop order is
                // reverse declaration order).
                let pool = global.db_client().await?;
                let mut continuation_upserts: Vec<_> = Vec::new();
                for (hier, continuation) in chunk.agent_instance_hierarchies() {
                    if let Some(c) = continuation {
                        continuation_upserts.push(
                            crate::db::agent_continuations::upsert(&pool, hier, c),
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
                let e = Error::Instance(e);
                note_error(
                    global,
                    scoped,
                    &conversation_tee,
                    identity.as_ref().map(|(h, _)| h.as_str()).or(registry.aih()),
                    last_response_id.as_deref(),
                    &e,
                )
                .await;
                Err(e)?;
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
                &global.db_client().await?, hier,
            )
            .await
            .unwrap_or(false);
            if !pending {
                break;
            }

            // Restart with the latest continuation only. No new
            // messages — the API picks up state from the
            // continuation token. Re-resolve the agent's laboratory
            // attachments too: one may have been attached or detached
            // while this pass ran, and each pass must dial whatever is
            // attached NOW.
            params.messages = Vec::new();
            params.continuation = last_continuation;
            params.laboratories = match resolve_laboratories(global, scoped, &lab_targets).await {
                Ok(laboratories) => laboratories,
                Err(e) => {
                    note_error(
                        global,
                        scoped,
                        &conversation_tee,
                        identity.as_ref().map(|(h, _)| h.as_str()).or(registry.aih()),
                        last_response_id.as_deref(),
                        &e,
                    )
                    .await;
                    Err(e)?;
                    unreachable!("Err(e)? diverges");
                }
            };
            // Record the ACTIVE set this pass will send.
            if let Err(e) = record_active_laboratories(
                global,
                identity.as_ref().map(|(h, _)| h.as_str()).or(registry.aih()),
                &params.laboratories,
            )
            .await
            {
                note_error(
                    global,
                    scoped,
                    &conversation_tee,
                    identity.as_ref().map(|(h, _)| h.as_str()).or(registry.aih()),
                    last_response_id.as_deref(),
                    &e,
                )
                .await;
                Err(e)?;
                unreachable!("Err(e)? diverges");
            }
        }
    }
}

/// Resolve an [`AgentRef`] into a typed agent. `Resolved` passes
/// through; `File` / `PythonInline` / `PythonFile` run their IO /
/// Python here via the shared 5-variant resolver (the `simple`
/// slot is never populated for agent refs — `--agent <ref>`
/// strings parse at the clap layer).
pub(crate) async fn resolve_agent_ref(
    global: &GlobalContext, scoped: &ScopedContext,
    agent: AgentRef,
) -> Result<InlineAgentBaseWithFallbacksOrRemoteCommitOptional, Error> {
    let (file, python_inline, python_file) = match agent {
        AgentRef::Resolved(resolved) => return Ok(resolved),
        AgentRef::File(p) => (Some(p), None, None),
        AgentRef::PythonInline(code) => (None, Some(code), None),
        AgentRef::PythonFile(p) => (None, None, Some(p)),
    };
    crate::source_resolver::resolve_source(
        global,
        scoped,
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

    use crate::context::{GlobalContext, ScopedContext};
    use crate::error::Error;

    pub async fn execute(_global: &GlobalContext, _scoped: &ScopedContext, _request: Request) -> Result<Response, Error> {
        Ok(objectiveai_sdk::cli::command::ResponseSchema(schemars::schema_for!(sdk::Request)))
    }
}

pub mod response_schema {
    use objectiveai_sdk::cli::command::agents::spawn as sdk;
    use objectiveai_sdk::cli::command::agents::spawn::response_schema::{Request, Response};

    use crate::context::{GlobalContext, ScopedContext};
    use crate::error::Error;

    pub async fn execute(_global: &GlobalContext, _scoped: &ScopedContext, _request: Request) -> Result<Response, Error> {
        Ok(objectiveai_sdk::cli::command::ResponseSchema(schemars::schema_for!(sdk::Response)))
    }
}
