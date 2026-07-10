//! `agents message` — unary delivery primitive with a total FIFO
//! order.
//!
//! The agent input is the shared [`AgentSelector`] — the same shape
//! `agents spawn` takes, so an unspawned agent can be messaged:
//!
//! - **ref**: nothing to lock or enqueue against (no AIH or tag
//!   exists before the spawn) — exec a detached `agents spawn` child
//!   (stream=true) carrying the resolved agent + message inline,
//!   return its first item as `Id`. The only route where content
//!   rides inline.
//! - **instance / BOUND tag** (the AIH lock) and **GROUPED tag**
//!   (the tag lock): ALWAYS enqueue first — the queue row's id fixes
//!   the message's delivery position at commit time — then loop:
//!   try_acquire the agent's whole lock family (all-or-nothing);
//!   won → if our row is STILL ACTIVE (probe first — a consumed row
//!   means we're done, not a reason to wake anyone), exec a WAKE-UP
//!   spawn child (EMPTY message — the agent's startup snapshot and
//!   pass-boundary reads drain the queue oldest-id-first) with the
//!   claim TRANSFERRED into it; either way race the ONE pinned
//!   `subscribe_delivered` (created before the loop so it survives
//!   iterations — our row flipped inactive → `Delivered`) against
//!   `wait_released` (the owner — possibly our own child — exited
//!   before consuming → re-race). Spawns are capped at
//!   [`MAX_WAKE_SPAWNS`]; past that the command errors loudly rather
//!   than respawning forever. This command never deletes its row: it
//!   is durable across crashes of any participant, and a spawn
//!   failure leaves it parked for a later `message` /
//!   `agents queue deliver`.
//!
//! FIFO: every message is a queue row and every spawn starts empty,
//! so delivery order is exactly queue-id (enqueue-commit) order
//! under any interleaving of senders, lock winners, and respawns.
//!
//! The message payload is resolved ONCE in this process (file IO /
//! Python run here). Fire-and-forget parking without the race lives
//! in `agents enqueue`.

use futures::StreamExt;
use objectiveai_sdk::agent::completions::message::RichContent;
use objectiveai_sdk::cli::command::agents::message::{Request, RequestMessage, Response};
use objectiveai_sdk::cli::command::agents::selector::{AgentRef, AgentSelector};
use objectiveai_sdk::cli::command::agents::spawn as spawn_sdk;
use objectiveai_sdk::cli::command::{BinaryExecutor, CommandExecutor};

use crate::context::Context;
use crate::error::Error;

/// Wake-spawn cap for one `agents message` wait. The rules:
///
/// - Each spawned wake child gets one full turn to consume our queue
///   row (its startup snapshot reads pending rows oldest-id-first, so
///   an active row IS read by the very first pass).
/// - A row still active after a spawned turn therefore signals a real
///   fault (lock not excluding, notify chain down, row unreadable) —
///   never a normal wait.
/// - After this many spawned children with the row still active, the
///   command errors out LOUDLY rather than respawning agents forever;
///   the row stays parked and durable for a later message/deliver.
const MAX_WAKE_SPAWNS: usize = 3;

pub async fn execute(ctx: &Context, request: Request) -> Result<Response, Error> {
    let Request {
        agent,
        message,
        dangerous_advanced,
        ..
    } = request;
    let seed = dangerous_advanced.as_ref().and_then(|a| a.seed);

    // Resolve the payload once, in this process.
    let content = resolve_message(ctx, message).await?;

    let state_dir = ctx.filesystem.state_dir();
    let route = match agent {
        AgentSelector::Ref { agent } => {
            // Resolve file/python refs HERE too — the child gets the
            // typed agent inline and never re-runs the Python.
            let resolved = super::spawn::resolve_agent_ref(ctx, agent).await?;
            Route::Ref {
                child: AgentSelector::Ref {
                    agent: AgentRef::Resolved(resolved),
                },
            }
        }
        AgentSelector::Instance {
            parent_agent_instance_hierarchy,
            agent_instance,
        } => {
            let parent = parent_agent_instance_hierarchy
                .as_deref()
                .unwrap_or(&ctx.config.agent_instance_hierarchy);
            instance_route(&state_dir, format!("{parent}/{agent_instance}"))
        }
        AgentSelector::Tag { agent_tag } => {
            match crate::db::tags::lookup(ctx.db_client().await?, &agent_tag).await? {
                crate::db::tags::LookupState::Bound {
                    agent_instance_hierarchy,
                } => instance_route(&state_dir, agent_instance_hierarchy),
                crate::db::tags::LookupState::Grouped { tag_group_id, .. } => {
                    let (dir, key) = super::locks::agent_tag_lock(&state_dir, &agent_tag);
                    Route::Locked {
                        dir,
                        key,
                        family: super::locks::Family::Group(tag_group_id),
                        hierarchy: None,
                        tag: Some(agent_tag.clone()),
                        child: AgentSelector::Tag { agent_tag },
                    }
                }
                crate::db::tags::LookupState::Absent => {
                    return Err(Error::TagNotFound(agent_tag));
                }
            }
        }
    };

    match route {
        Route::Ref { child } => spawn_child(child, content, seed, Vec::new()).await,
        Route::Locked {
            dir,
            key,
            family,
            hierarchy,
            tag,
            child,
        } => {
            // Park the message FIRST — the row's id fixes its delivery
            // position at commit time, before any lock race. Rows are
            // consumed strictly oldest-id-first, so whoever wins any
            // race below, delivery order stays enqueue order.
            let queue_id = crate::db::message_queue::enqueue_with_content(
                ctx.db_client().await?,
                hierarchy,
                tag,
                &ctx.config.agent_instance_hierarchy,
                None,
                content,
            )
            .await?;
            let pool = ctx.db_client().await?.clone();
            // ONE delivery subscription for the whole wait, pinned OUTSIDE
            // the loop: its LISTEN attach + probe make progress across
            // iterations instead of being torn down and restarted by every
            // `wait_released` wake — a recreated-per-iteration subscription
            // can be starved indefinitely by a busy released arm (the
            // ghost-respawn incident), a pinned one cannot.
            let delivered =
                crate::db::message_queue::subscribe_delivered(&pool, queue_id);
            tokio::pin!(delivered);
            let mut wake_spawns = 0usize;
            loop {
                // FETCH the current family + LOCK it (all-or-nothing) in one
                // call — the membership can shift during a long wait, so it
                // must be resolved at lock time. Won → re-probe OUR row
                // first: if it was already consumed (we won the lock before
                // observing the flip), release the family and resolve —
                // spawning here would be a pointless empty wake turn. Still
                // active → exec a WAKE-UP spawn child (EMPTY message; the
                // child drains the queue) with the family transferred, then
                // fall into the same race as a loser: our row flipping
                // inactive is the resolution either way. A spawn failure
                // propagates with the row left parked — durable, recoverable
                // by a later message/deliver.
                if let Some(fam) = super::locks::try_acquire_family(
                    ctx.agent_locks(),
                    ctx.db_client().await?,
                    &state_dir,
                    family.clone(),
                )
                .await?
                {
                    if !crate::db::message_queue::is_active(&pool, queue_id).await? {
                        for lock in fam.into_locks() {
                            lock.release().map_err(|e| Error::Lockfile {
                                key: key.clone(),
                                source: e,
                            })?;
                        }
                        return Ok(Response::Delivered);
                    }
                    // Respawn bound: each spawned child gets one full turn to
                    // consume our row (its startup snapshot reads the queue
                    // oldest-id-first, so an active row IS read). Needing more
                    // than MAX_WAKE_SPAWNS turns means something is broken —
                    // lock not actually excluding, notify chain down, row
                    // unreadable — and looping further would respawn agents
                    // forever (the incident). Error out loudly instead; the
                    // row stays parked and durable.
                    if wake_spawns >= MAX_WAKE_SPAWNS {
                        return Err(Error::Instance(format!(
                            "agents message: queue row {queue_id} still undelivered \
                             after {MAX_WAKE_SPAWNS} wake spawns — aborting instead \
                             of respawning forever; the message remains parked"
                        )));
                    }
                    wake_spawns += 1;
                    spawn_child(
                        child.clone(),
                        RichContent::Text(String::new()),
                        seed,
                        fam.into_locks(),
                    )
                    .await?;
                }
                tokio::select! {
                    delivery = &mut delivered => {
                        delivery?;
                        return Ok(Response::Delivered);
                    }
                    // Wait for the owner — possibly our own child — to EXIT
                    // (release, don't acquire), so the fetch+lock above sees
                    // a free family on the next iteration.
                    released = objectiveai_sdk::lockfile::wait_released(&dir, &key) => {
                        released.map_err(|e| Error::Lockfile {
                            key: key.clone(),
                            source: e,
                        })?;
                    }
                }
            }
        }
    }
}

/// Resolved delivery route for the non-enqueue flow.
enum Route {
    /// Plain agent ref — nothing to lock or enqueue against; always
    /// spawns a fresh agent carrying the message.
    Ref { child: AgentSelector },
    /// Lockable target: the PRIMARY lock coordinates, the agent's whole
    /// lock `family` (acquired together so none of its tags/labs can be
    /// relocated/detached while live), the queue target (exactly one of
    /// `hierarchy` / `tag` is `Some`), and the selector the spawn child receives.
    Locked {
        dir: std::path::PathBuf,
        key: String,
        family: super::locks::Family,
        hierarchy: Option<String>,
        tag: Option<String>,
        child: AgentSelector,
    },
}

/// Route for a fully resolved `agent_instance_hierarchy`: the AIH
/// lock, queued against the hierarchy, child re-addressed as an
/// explicit Instance (parent + leaf) so it doesn't depend on the
/// child process's own identity.
fn instance_route(state_dir: &std::path::Path, hierarchy: String) -> Route {
    let (dir, key) = super::locks::agent_instance_lock(state_dir, &hierarchy);
    let child = AgentSelector::Instance {
        parent_agent_instance_hierarchy: Some(
            crate::db::tags::parent_of(&hierarchy).to_string(),
        ),
        agent_instance: crate::db::tags::leaf_of(&hierarchy).to_string(),
    };
    Route::Locked {
        dir,
        key,
        family: super::locks::Family::Hierarchy(hierarchy.clone()),
        hierarchy: Some(hierarchy),
        tag: None,
        child,
    }
}

/// Exec a detached `agents spawn` child (stream=true) and return its
/// first item as the unary response. The Ref route passes real
/// `content`; the Locked route passes EMPTY content — `agents spawn`
/// maps that to an empty `messages` array, a wake-up turn whose
/// prompt is the queue drain. When `transfer` is non-empty, the won
/// claims — AIH or TAG — are TRANSFERRED into the child: it adopts
/// each inherited lock at startup and re-acquires it instantly,
/// becoming the sole owner, and the lock lives exactly as long as the
/// child. (Continuous hold through the child's pre-first-chunk window
/// is preserved — the transfer hands off the same OS handles with no
/// gap; a BOUND tag routes by AIH and never re-acquires the tag lock,
/// so holding it for the child's lifetime is safe.) When empty (a
/// plain agent ref), the child acquires its own lock fresh. The
/// child's first item is always its `Id` (chunks are gated behind
/// it); the rest of the stream is dropped and the orphan keeps
/// running.
async fn spawn_child(
    agent: AgentSelector,
    content: RichContent,
    seed: Option<i64>,
    transfer: Vec<super::locks::AgentLock>,
) -> Result<Response, Error> {
    let child_request = spawn_sdk::Request {
        path_type: spawn_sdk::Path::AgentsSpawn,
        message: RequestMessage::Inline(content),
        agent,
        dangerous_advanced: Some(spawn_sdk::RequestDangerousAdvanced {
            stream: Some(true),
            seed,
        }),
        base: Default::default(),
    };

    let exe = std::env::current_exe()
        .map_err(|e| Error::Spawn("current_exe".into(), e))?;
    let mut executor = BinaryExecutor::from_path(exe).detach(true);
    // Hold the in-process guards across the synchronous prepare→spawn→transfer
    // inside `execute` (each cross-process claim is handed to the child). The
    // `AgentLock`s — now guard-only after `take_claim` — drop at the end of this
    // fn, freeing the per-key in-process mutexes; by then the child owns the
    // cross-process locks, so a later in-process acquirer passes the mutex but
    // correctly fails the lockfile.
    let mut transfer = transfer;
    let claims: Vec<_> = transfer
        .iter_mut()
        .filter_map(|lock| lock.take_claim())
        .collect();
    if !claims.is_empty() {
        executor = executor.transfer_locks(claims);
    }
    let mut stream = executor
        .execute::<spawn_sdk::Request, spawn_sdk::ResponseItem>(child_request, None)
        .await
        .map_err(|e| Error::Instance(format!("exec agents spawn child: {e}")))?;
    let first = stream
        .next()
        .await
        .ok_or(Error::EmptyStream)?
        .map_err(|e| Error::Instance(format!("exec agents spawn child: {e}")))?;
    match first {
        spawn_sdk::ResponseItem::Id(agent_instance_hierarchy) => Ok(Response::Id {
            agent_instance_hierarchy,
        }),
        spawn_sdk::ResponseItem::Chunk(_) => Err(Error::Instance(
            "agents spawn child emitted a chunk before its id".to_string(),
        )),
    }
}

pub async fn resolve_message(
    ctx: &Context,
    message: RequestMessage,
) -> Result<RichContent, Error> {
    let (simple, inline, file, python_inline, python_file) = match message {
        RequestMessage::Inline(rich) => return Ok(rich),
        RequestMessage::Simple(s) => (Some(s), None, None, None, None),
        RequestMessage::File(p) => (None, None, Some(p), None, None),
        RequestMessage::PythonInline(code) => (None, None, None, Some(code), None),
        RequestMessage::PythonFile(p) => (None, None, None, None, Some(p)),
    };
    crate::source_resolver::resolve_source(
        ctx,
        simple,
        inline,
        file,
        python_inline,
        python_file,
        RichContent::Text,
    )
    .await
}

pub mod request_schema {
    use objectiveai_sdk::cli::command::agents::message as sdk;
    use objectiveai_sdk::cli::command::agents::message::request_schema::{Request, Response};

    use crate::context::Context;
    use crate::error::Error;

    pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
        Ok(objectiveai_sdk::cli::command::ResponseSchema(schemars::schema_for!(sdk::Request)))
    }
}

pub mod response_schema {
    use objectiveai_sdk::cli::command::agents::message as sdk;
    use objectiveai_sdk::cli::command::agents::message::response_schema::{Request, Response};

    use crate::context::Context;
    use crate::error::Error;

    pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
        Ok(objectiveai_sdk::cli::command::ResponseSchema(schemars::schema_for!(sdk::Response)))
    }
}
